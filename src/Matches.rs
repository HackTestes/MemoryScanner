
use crate::GenericOSInterface;

// It represents the matches related to a particular memory region
pub struct Matches
{
    mem_region: GenericOSInterface::GenericMemoryRegion,

    // Remember that all of the matches are RELATIVE to the region!
    matches: Vec<usize>
}

pub enum MatchDisplayStyle
{
    Decimal,
    Hex
}

impl Matches
{
    pub fn new(memory_region: GenericOSInterface::GenericMemoryRegion, matches_addresses: Vec<usize>) -> Matches
    {
        return Matches
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
            // 16 -> minimum of 16 characters
            addresses_in_hex.push( format!("{:#016X}", addr) );
        }

        return addresses_in_hex;
    }

    pub fn display_matches(&self, display_style: MatchDisplayStyle) -> String
    {
        // Memory region info
        // TODO implement Display on those types
        let mut mem_region_string = "".to_string();

        mem_region_string += format!("Memory region \n").as_str();
        mem_region_string += format!("-> Permissions: {} \n", self.mem_region.permissions).as_str();
        mem_region_string += format!("-> State: {} \n", self.mem_region.state).as_str();
        mem_region_string += format!("-> Size (bytes): {} \n", self.mem_region.size_bytes).as_str();
        mem_region_string += format!("-> Base address: {} \n", self.mem_region.base_address).as_str();
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

    // Does the attach method check for errors and return the handle on success?
    #[test]
    fn TestMatches()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            0,
            1000);

        let match_obj = Matches::new(memory_region, vec![0, 10, 25, 15] );

        println!("{}", match_obj.display_matches(MatchDisplayStyle::Hex));
        assert!( false );
    }
}