
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
    pub fn get_absolute_addresses(&self) -> Vec<usize>
    {
        let mut absulute_addresses: Vec<usize> = vec![];

        for match_addr in self.matches
        {
            absulute_addresses.push( match_addr + self.mem_region.base_address );
        }

        return absulute_addresses;
    }

    pub fn get_absolute_addresses_decimal -> Vec<String>
    {
        let addresses = self.get_absolute_addresses();
        let addresses_in_decimal: Vec<String> = Vec::with_capacity( addresses.len() );

        for addr in addresses
        {
            addresses_in_decimal.push( format!("{}", addr) );
        }

        return addresses_in_decimal;
    }

    pub fn get_absolute_addresses_hex -> Vec<String>
    {
        let addresses = self.get_absolute_addresses();
        let addresses_in_hex: Vec<String> = Vec::with_capacity( addresses.len() );

        for addr in addresses
        {
            addresses_in_hex.push( format!("{:02X}", addr) );
        }

        return addresses_in_decimal;
    }

    pub fn display_matches(display_style: MatchDisplayStyle) -> String
    {
        // Memory region info
        // TODO implement Display on those types
        let mem_region_string = format!("
            Memory region \
            -> Permission: {} \
            -> State: {} \
            -> Size (bytes): {} \
            -> Base address: {} \n",
            self.mem_region.permissions,
            self.mem_region.state
            self.mem_region.size_bytes
            self.mem_region.base_address);

        // Addresses
        // Get absolute addresses
        let absulute_addresses = self.get_absolute_addresses();

        // Select display style: hex, decimal
        let display_addr_values = match display_style
        {
            // Format the string in the decimal style
            MatchDisplayStyle:Decimal => self.get_absolute_addresses_decimal(),

            // Format the string in the hex style
            MatchDisplayStyle:Hex => self.get_absolute_addresses_decimal()
        };

        let addresses_string = "".to_string();

        for idx in 0..display_addr_values.len()
        {
            addresses_string.push( format!("{}: {}\n", idx, display_addr_values[idx]) );
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
        assert!( false );
    }
}