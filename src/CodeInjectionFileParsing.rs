use std::num::ParseIntError;

// Code injection file format
/*

// Comments are just like in Rust

// module: takes the name of a file (.elf, .exe, .dll, .so) and look for its content in memory
// exe_memory: searches in all executable memory (useful for programs that do JIT code - .NET, JS, Python...)
search_type = module OR exe_memory

// Only needed for module searches
module_name = something.exe

// Entries
instruction = 0x9090, matches_allowed = 1 // Search and replace the entirety of the instruction

instruction = 0x90909090, range = 0:2, matches_allowed = 1 // Search for this instruction, but only replace a range (start position:length in bytes)

// Positions
// [0] = AA
// [1] = AA
// [2] = 90
// [3] = 90
instruction = 0xAAAA9090, range = 0:2, matches_allowed = 1 // Replace the first 2 bytes ("0xAAAA")

instruction = 0xAAAA9090, range = 2:2, matches_allowed = 1 // Replace the last 2 bytes ("0x9090")

instruction = 0xAAAA9090, range = 0:2, matches_allowed = 3 // Sometimes we need to replace the same instruction in many places

instruction = 0xAAAA9090, range = 0:2, matches_allowed = 0 // Sometimes we just want to debug things

*/

#[derive(Debug)]
pub enum InjectionFileParsingErrors
{
    // Failure during entry parsing
    EntryParsingError_UnknownFiled,
    EntryParsingError_InvalidParameter, // Let's say, an incompatible type
    EntryParsingError_InvalidRange,
    EntryParsingError_InvalidRangeValue, // Valid range format, but it uses letters or negative values
    EntryParsingError_InstructionMissingBytes, // 0x9 instead of 0x90
    EntryParsingError_MissingField,
    EntryParsingError_MissingKeyValuePair,
    EntryParsingError_NotAHex,
    EntryParsingError_OddInstruction,
    EntryParsingError_InvalidHexInstruction,
    EntryParsingError_EmptyInstruction,
    EntryParsingError_UnknownSearchType,

    // Error for duplicated input
    EntryParsingError_Duplicates,

    EntryParsingError_MixingEntryTypes,

    // Failure in validation
    ValidationError_NoSearchType,
    ValidationError_NoModuleName,
    ValidationError_NoInstructions
}

#[derive(PartialEq)]
#[derive(Debug)]
pub enum SearchType
{
    module_name,
    exe_memory
}

#[derive(Debug)]
struct InjectionEntry
{
    pub instruction: Vec<u8>,
    pub range: Option<(usize, usize)>,
    pub matches_allowed: usize
}

impl InjectionEntry
{
    pub fn new(input_instruction: Vec<u8>, input_range: Option<(usize, usize)>, input_matches_allowed: usize) -> InjectionEntry
    {
        return InjectionEntry
        {
            instruction: input_instruction,
            range: input_range,
            matches_allowed: input_matches_allowed
        };
    }

    pub fn empty() -> InjectionEntry
    {
        return InjectionEntry
        {
            instruction: vec![],
            range: None,
            matches_allowed: 0
        };
    }
}

#[derive(Debug)]
struct InjectionConfiguration
{
    pub search_type: Option<SearchType>,
    pub module_name: Option<String>,
    pub instructions: Vec<InjectionEntry>,
}

impl InjectionConfiguration
{
    pub fn new() -> InjectionConfiguration
    {
        return InjectionConfiguration
        {
            search_type: None,
            module_name: None,
            instructions: vec![]
        };
    }
}

fn is_line_empty(line_content: &str) -> bool
{
    // Remove any white space and check if simply and empty String
    return "" == line_content.trim();
}

fn remove_comments(line_content: &str) -> String
{
    // ???????????????
    return line_content.split("//").collect::<Vec<_>>()[0].to_string();
}

// https://stackoverflow.com/questions/52987181/how-can-i-convert-a-hex-string-to-a-u8-slice
pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError>
{
    let mut hex_list: Vec<u8> = vec![];
    for i in (0..s.len()).step_by(2)
    {
        let hex_byte = &s[i..i + 2];

        // We can't decode it
        let hex_doceded_r = u8::from_str_radix(&s[i..i + 2], 16);
        if hex_doceded_r.is_err()
        {
            return Err(hex_doceded_r.unwrap_err());
        }
        else
        {
            hex_list.push(hex_doceded_r.unwrap());
        }
    }

    return Ok(hex_list);
}

fn parse_entry(line_content: String, injection_config: &mut InjectionConfiguration, line_num: usize) -> Result<(), InjectionFileParsingErrors>
{
    // Get each field
    let fields = line_content.split(",");

    // Start with an empty entry and populate it as we read the line
    let mut injection_entry = InjectionEntry::empty();

    // This checks if we are not mixing a regular instruction entry with module names or search types
    let mut is_injection_entry: bool = false;

    // For each field, get its parameters
    for field in fields
    {
        let items: Vec<&str> = field.split("=").collect();

        // Check for key value pairs
        if items.len() != 2
        {
            eprintln!("Error on line {} - Missing key value pair: {}", line_num, field);
            return Err(InjectionFileParsingErrors::EntryParsingError_MissingKeyValuePair);
        }

        let key = items[0].trim();
        let value = items[1].trim();

        // Check for known keys

        match key 
        {
            "search_type" => {

                // For when we mix different types of things in the same line
                if is_injection_entry == true
                {
                    eprintln!("Error on line {} - Mixing entry types on the same line: {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_MixingEntryTypes); 
                }

                if injection_config.search_type != None
                {
                    eprintln!("Error on line {} - Duplicated search type: {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_Duplicates); 
                }

                match value
                {
                    "module" => { injection_config.search_type = Some(SearchType::module_name); },
                    "exe_memory" => { injection_config.search_type = Some(SearchType::exe_memory); },
                    _ => {
                        eprintln!("Error on line {} - Unknown search type: {}", line_num, value);
                        return Err(InjectionFileParsingErrors::EntryParsingError_UnknownSearchType);
                    }
                }
            },

            "module_name" => {

                // Since any name is valid, there isn't much to check

                // Except for duplicate entries
                // We can only store this value if the was no previous one
                if injection_config.module_name != None
                {
                    eprintln!("Error on line {} - Duplicated module name: {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_Duplicates); 
                }

                // Or if we are mixing things
                if is_injection_entry == true
                {
                    eprintln!("Error on line {} - Mixing entry types on the same line: {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_MixingEntryTypes); 
                }

                injection_config.module_name = Some(value.to_string());
            },

            "instruction" => {

                // Enable the injection flag
                is_injection_entry = true;

                // Check if is a hex value
                if value[0..2] != *"0x"
                {
                    eprintln!("Error on line {} - Not on the hex format (0x...): {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_NotAHex);
                }
 
                // Remove the "0x" and see if we have all bytes (aka, we don't have odd number of characters)
                let pure_hex = &value[2..];
                if pure_hex.len() % 2 != 0
                {
                    eprintln!("Error on line {} - Invalid Hex instruction (odd number): {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_OddInstruction);
                }

                // Can we decode it?
                if decode_hex(&pure_hex).is_err()
                {
                    eprintln!("Error on line {} - Invalid Hex instruction: {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidHexInstruction);
                }
                
                // Everythin is ok, decode it and store it
                let instruction: Vec<u8> = decode_hex(&pure_hex).unwrap();

                injection_entry.instruction = instruction;
            },

            "range" => {

                // Enable the injection flag
                is_injection_entry = true;

                let range_items: Vec<&str> = value.split(":").collect();

                if range_items.len() != 2
                {
                    eprintln!("Error on line {} - Invalid range parameter: {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidRange);
                }

                let start_string = range_items[0].trim();
                let size_string = range_items[1].trim();

                if start_string.parse::<usize>().is_err()
                {
                    eprintln!("Error on line {} - Invalid range start: {}", line_num, start_string);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue);
                }

                if size_string.parse::<usize>().is_err()
                {
                    eprintln!("Error on line {} - Invalid range size: {}", line_num, size_string);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue);
                }

                let start = start_string.parse::<usize>().unwrap();
                let size = size_string.parse::<usize>().unwrap();

                // The size cannot be zero, it does not make sense to replace 0 bytes of code
                if size == 0
                {
                    eprintln!("Error on line {} - Invalid range size (it cannot be zero): {}", line_num, size);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue);
                }

                // Everything is ok, so store it
                injection_entry.range = Some((start, size));
                
            },

            "matches_allowed" => {

                // Enable the injection flag
                is_injection_entry = true;

                // Is the value a valid number? 
                if value.parse::<usize>().is_ok()
                {
                    // Yes, then store it
                    injection_entry.matches_allowed = value.parse::<usize>().unwrap();
                }
                else
                {
                    eprintln!("Error on line {} - Invalid matches allowed number: {}", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidParameter);
                }
            },

            _ => {
                eprintln!("Error on line {} - Unknown field: {}", line_num, key);
                return Err(InjectionFileParsingErrors::EntryParsingError_UnknownFiled);
            },
        }

    }

    // Validate entry

    // In case of an injection entry
    if is_injection_entry == true
    {
        // Validate every setting
        if injection_entry.instruction.len() == 0
        {
            eprintln!("Error on line {} - Empty Instruction", line_num);
            return Err(InjectionFileParsingErrors::EntryParsingError_EmptyInstruction);
        }

        injection_config.instructions.push( injection_entry );
    }

    // Everything else we can rely on previous checks
    return Ok(());
}

fn parse_injection_file(injection_file_contents: String) -> Result<InjectionConfiguration, InjectionFileParsingErrors>
{
    // Start with an empty config. We will change it during the parsing process
    let mut injection_config = InjectionConfiguration::new();

    // Get each line
    let lines: Vec<&str> = injection_file_contents.split("\n").collect();

    for (line_num, line_content) in lines.iter().enumerate()
    {
        // Skip empty lines
        if is_line_empty(line_content) == true
        {
            continue;
        }

        // Remove comments from line
        let entry_string = remove_comments(line_content);

            // Is the line just a comment? If so, skip it
            if entry_string.len() == 0
            {
                continue;
            }

        // Interpret the entry and store it
        let parsing_result = parse_entry(entry_string, &mut injection_config, line_num);

        if parsing_result.is_err()
        {
            // Return the error up in the stack
            return Err(parsing_result.unwrap_err());
        }
    }

    // Validate final settings. Is there anything missing that we can't continue without?

    // Does it have a search type?
    if injection_config.search_type == None
    {
        eprintln!("Parsing error: there is no search type, please add one");
        return Err(InjectionFileParsingErrors::ValidationError_NoSearchType);
    }  

    // If it is a module search, does it contain one?
    if injection_config.search_type == Some(SearchType::module_name) && injection_config.module_name == None
    {
        eprintln!("Parsing error: there is no module name, please add one");
        return Err(InjectionFileParsingErrors::ValidationError_NoModuleName); 
    }

    // Is there any instructions to replace?
    if injection_config.instructions.len() == 0
    {
        eprintln!("Parsing error: there are no instructions to search for, please add some");
        return Err(InjectionFileParsingErrors::ValidationError_NoInstructions); 
    }

    return Ok(injection_config);
}

#[cfg(test)]
mod tests
{
    use crate::CodeInjectionFileParsing::*;

    const example_test_RegularCase: &str = "    
    
// Comments are just like in Rust

// module: takes the name of a file (.elf, .exe, .dll, .so) and look for its content in memory
// exe_memory: searches in all executable memory (useful for programs that do JIT code - .NET, JS, Python...)
search_type = module

// Only needed for module searches
module_name = something.exe

// Entries
instruction = 0x9090, matches_allowed = 1 // Search and replace the entirety of the instruction
instruction = 0x90909090, range = 0:2, matches_allowed = 1 // Search for this instruction, but only replace a range (start position:length in bytes)
instruction = 0xAAAA9090, range = 0:2, matches_allowed = 1 // Replace the first 2 bytes (\"0xAAAA\")

    ";

    #[test]
    fn InjectionFileParsing_RegularCase()
    {
        println!("{:?}", parse_injection_file(example_test_RegularCase.to_string()));
        assert_eq!(true, false);
    }
}