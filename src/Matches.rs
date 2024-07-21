use std::mem::size_of;
use crate::GenericOSInterface;

// It represents the matches related to a particular memory region
#[derive(Clone)]
pub struct AddressMatches
{
    pub mem_region: GenericOSInterface::GenericMemoryRegion,

    // Remember that all of the matches are RELATIVE to the region!
    pub matches: Vec<usize>
}

pub enum MatchDisplayStyle
{
    Decimal,
    Hex
}

impl AddressMatches
{
    pub fn new(memory_region: GenericOSInterface::GenericMemoryRegion, matches_addresses: Vec<usize>) -> AddressMatches
    {
        return AddressMatches
        {
            mem_region: memory_region,
            matches: matches_addresses
        };
    }

    pub fn get_absolute_addresses(&self) -> Vec<usize>
    {
        let mut absulute_addresses: Vec<usize> = vec![];

        for match_addr in &self.matches
        {
            absulute_addresses.push( match_addr + self.mem_region.base_address );
        }

        return absulute_addresses;
    }

    pub fn get_absolute_addresses_decimal(&self) -> Vec<String>
    {
        let addresses = self.get_absolute_addresses();
        let mut addresses_in_decimal: Vec<String> = Vec::with_capacity( addresses.len() );

        for addr in addresses
        {
            addresses_in_decimal.push( format!("{}", addr) );
        }

        return addresses_in_decimal;
    }

    pub fn get_absolute_addresses_hex(&self) -> Vec<String>
    {
        let addresses = self.get_absolute_addresses();
        let mut addresses_in_hex: Vec<String> = Vec::with_capacity( addresses.len() );

        for addr in addresses
        {
            // : -> indicates an alternative mode
            // # -> selects the mode
            // X -> high case hex
            // 0 -> it has trailing zeros
            // 1$ -> the minimum num of chars is given to the function (in this case, it is based on the size of usize)
            //  Basing the representation on the size of usize is more portable
            // * 2: each byte is represented by 2 hex characters
            // + 2: the "0x" was also counted in, se we offset it
            addresses_in_hex.push( format!("{:#01$X}", addr, (size_of::<usize>()) * 2 + 2) );
        }

        return addresses_in_hex;
    }

    pub fn display_matches(&self, display_style: MatchDisplayStyle) -> String
    {
        // Memory region info
        let mut mem_region_string = "".to_string();

        mem_region_string += format!("Memory region \n").as_str();
        mem_region_string += format!("-> Permissions: {} \n", self.mem_region.permissions).as_str();
        mem_region_string += format!("-> State: {} \n", self.mem_region.state).as_str();
        mem_region_string += format!("-> Base address: {} \n", self.mem_region.base_address).as_str();
        mem_region_string += format!("-> Size (bytes): {} \n", self.mem_region.size_bytes).as_str();
        mem_region_string += format!("\n").as_str();

        // Addresses
        // Get absolute addresses
        let absulute_addresses = self.get_absolute_addresses();
        let mut addresses_string: String = "".to_string();

        // Select display style: hex, decimal
        let display_addr_values = match display_style
        {
            // Format the string in the decimal style
            MatchDisplayStyle::Decimal =>
            {
                addresses_string += format!("Index: Address in decimal \n {} \n", "-".repeat(18)).as_str();
                self.get_absolute_addresses_decimal()
            },

            // Format the string in the hex style
            MatchDisplayStyle::Hex =>
            {
                addresses_string += format!("Index: Address in hex \n {} \n", "-".repeat(18)).as_str();
                self.get_absolute_addresses_hex()
            }
        };

        for idx in 0..display_addr_values.len()
        {
            addresses_string += format!("{}: {}\n", idx, display_addr_values[idx]).as_str();
        }

        // Put everything together and return
        return format!("{}{}", mem_region_string, addresses_string);
    }
}


#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::Matches::*;

    #[ignore]
    #[test]
    fn TestMatches()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            0,
            1000);

        let match_obj = AddressMatches::new(memory_region, vec![0, 10, 25, 15] );

        println!("{}", match_obj.display_matches(MatchDisplayStyle::Hex));
        assert!( false );
    }

    #[test]
    fn TestMatches_AbsoluteAddressCalculation()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            0,
            1000);

        let match_obj = AddressMatches::new(memory_region, vec![0, 10, 25, 15] );

        assert_eq!( match_obj.get_absolute_addresses(), vec![0, 10, 25, 15] );

        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            100,
            1000);

        let match_obj = AddressMatches::new(memory_region, vec![0, 10, 25, 15] );

        assert_eq!( match_obj.get_absolute_addresses(), vec![100, 110, 125, 115] );
    }

    #[test]
    fn TestMatches_AbsoluteAddress_HexStyle()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            0,
            1000);

        let match_obj = AddressMatches::new(memory_region, vec![0, 1, 255] );

        assert_eq!( match_obj.get_absolute_addresses_hex(), vec!["0x0000000000000000".to_string(), "0x0000000000000001".to_string(), "0x00000000000000FF".to_string()] );
    }

    #[test]
    fn TestMatches_AbsoluteAddress_DecimalStyle()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            0,
            1000);

        let match_obj = AddressMatches::new(memory_region, vec![0, 1, 255] );

        assert_eq!( match_obj.get_absolute_addresses_decimal(), vec!["0".to_string(), "1".to_string(), "255".to_string()] );
    }
}