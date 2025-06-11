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
#[derive(PartialEq)]
pub enum InjectionFileParsingErrors
{
    // Failure during entry parsing
    EntryParsingError_UnknownFiled,
    EntryParsingError_InvalidParameter, // Let's say, an incompatible type
    EntryParsingError_InvalidRangeValue, // Valid range format, but it uses letters or negative values
    EntryParsingError_InvalidRange_MissingPair,
    EntryParsingError_MissingKeyValuePair,
    EntryParsingError_NotAHex,
    EntryParsingError_OddInstruction, // 0x9 instead of 0x90
    EntryParsingError_InvalidHexInstruction,
    EntryParsingError_EmptyInstruction,
    EntryParsingError_EmptyMatches,
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
#[derive(PartialEq)]
struct InjectionEntry
{
    pub instruction: Vec<u8>,
    pub range: Option<(usize, usize)>,
    pub matches_allowed: Option<usize>
}

impl InjectionEntry
{
    pub fn new(input_instruction: Vec<u8>, input_range: Option<(usize, usize)>, input_matches_allowed: usize) -> InjectionEntry
    {
        return InjectionEntry
        {
            instruction: input_instruction,
            range: input_range,
            matches_allowed: Some(input_matches_allowed)
        };
    }

    pub fn empty() -> InjectionEntry
    {
        return InjectionEntry
        {
            instruction: vec![],
            range: None,
            matches_allowed: None
        };
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
struct InjectionConfiguration
{
    pub search_type: Option<SearchType>,
    pub module_name: Option<String>,
    pub instructions: Vec<InjectionEntry>,
}

impl InjectionConfiguration
{
    pub fn new(search_type_input: Option<SearchType>, module_name_input: Option<String>, instructions_input: Vec<InjectionEntry>) -> InjectionConfiguration
    {
        return InjectionConfiguration
        {
            search_type: search_type_input,
            module_name: module_name_input,
            instructions: instructions_input
        };
    }

    pub fn empty() -> InjectionConfiguration
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
            eprintln!("Error on line {} - Missing key value pair: \"{}\"", line_num, field);
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
                    eprintln!("Error on line {} - Mixing entry types on the same line: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_MixingEntryTypes); 
                }

                if injection_config.search_type != None
                {
                    eprintln!("Error on line {} - Duplicated search type: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_Duplicates); 
                }

                match value
                {
                    "module" => { injection_config.search_type = Some(SearchType::module_name); },
                    "exe_memory" => { injection_config.search_type = Some(SearchType::exe_memory); },
                    _ => {
                        eprintln!("Error on line {} - Unknown search type: \"{}\"", line_num, value);
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
                    eprintln!("Error on line {} - Duplicated module name: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_Duplicates); 
                }

                // Or if we are mixing things
                if is_injection_entry == true
                {
                    eprintln!("Error on line {} - Mixing entry types on the same line: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_MixingEntryTypes); 
                }

                injection_config.module_name = Some(value.to_string());
            },

            "instruction" => {

                // Enable the injection flag
                is_injection_entry = true;

                if injection_entry.instruction.len() != 0
                {
                    eprintln!("Error on line {} - Duplicated instruction: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_Duplicates); 
                }

                // Check if is a hex value
                if value[0..2] != *"0x"
                {
                    eprintln!("Error on line {} - Not on the hex format (0x...): \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_NotAHex);
                }
 
                // Remove the "0x" and see if we have all bytes (aka, we don't have odd number of characters)
                let pure_hex = &value[2..];
                if pure_hex.len() % 2 != 0
                {
                    eprintln!("Error on line {} - Invalid Hex instruction (odd number): \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_OddInstruction);
                }

                // Can we decode it?
                if decode_hex(&pure_hex).is_err()
                {
                    eprintln!("Error on line {} - Invalid Hex instruction: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidHexInstruction);
                }
                
                // Everythin is ok, decode it and store it
                let instruction: Vec<u8> = decode_hex(&pure_hex).unwrap();

                injection_entry.instruction = instruction;
            },

            "range" => {

                // Enable the injection flag
                is_injection_entry = true;

                if injection_entry.range != None
                {
                    eprintln!("Error on line {} - Duplicated range: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_Duplicates); 
                }

                let range_items: Vec<&str> = value.split(":").collect();

                if range_items.len() != 2
                {
                    eprintln!("Error on line {} - Invalid range parameter (missing pair \"start:size\"): \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidRange_MissingPair);
                }

                let start_string = range_items[0].trim();
                let size_string = range_items[1].trim();

                if start_string.parse::<usize>().is_err()
                {
                    eprintln!("Error on line {} - Invalid range start: \"{}\"", line_num, start_string);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue);
                }

                if size_string.parse::<usize>().is_err()
                {
                    eprintln!("Error on line {} - Invalid range size: \"{}\"", line_num, size_string);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue);
                }

                let start = start_string.parse::<usize>().unwrap();
                let size = size_string.parse::<usize>().unwrap();

                // The size cannot be zero, it does not make sense to replace 0 bytes of code
                if size == 0
                {
                    eprintln!("Error on line {} - Invalid range size (it cannot be zero): \"{}\"", line_num, size);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue);
                }

                // Everything is ok, so store it
                injection_entry.range = Some((start, size));
                
            },

            "matches_allowed" => {

                // Enable the injection flag
                is_injection_entry = true;

                if injection_entry.matches_allowed != None
                {
                    eprintln!("Error on line {} - Duplicated allowed matches: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_Duplicates); 
                }

                // Is the value a valid number? 
                if value.parse::<usize>().is_ok()
                {
                    // Yes, then store it
                    injection_entry.matches_allowed = Some(value.parse::<usize>().unwrap());
                }
                else
                {
                    eprintln!("Error on line {} - Invalid matches allowed number: \"{}\"", line_num, value);
                    return Err(InjectionFileParsingErrors::EntryParsingError_InvalidParameter);
                }
            },

            _ => {
                eprintln!("Error on line {} - Unknown field: \"{}\"", line_num, key);
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

        if injection_entry.matches_allowed == None
        {
            eprintln!("Error on line {} - Empty allowed matches", line_num);
            return Err(InjectionFileParsingErrors::EntryParsingError_EmptyMatches);
        }

        injection_config.instructions.push( injection_entry );
    }

    // Everything else we can rely on previous checks
    return Ok(());
}

fn parse_injection_file(injection_file_contents: String) -> Result<InjectionConfiguration, InjectionFileParsingErrors>
{
    // Start with an empty config. We will change it during the parsing process
    let mut injection_config = InjectionConfiguration::empty();

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

    const example_test_RegularCase_ExeMemory: &str = "    
    
// Comments are just like in Rust

// module: takes the name of a file (.elf, .exe, .dll, .so) and look for its content in memory
// exe_memory: searches in all executable memory (useful for programs that do JIT code - .NET, JS, Python...)
search_type = exe_memory

// Entries
instruction = 0x9090, matches_allowed = 1 // Search and replace the entirety of the instruction
instruction = 0x90909090, range = 0:2, matches_allowed = 1 // Search for this instruction, but only replace a range (start position:length in bytes)
instruction = 0xAAAA9090, range = 0:2, matches_allowed = 1 // Replace the first 2 bytes (\"0xAAAA\")

    ";

    #[test]
    fn InjectionFileParsing_RegularCase()
    {
        let parsing_result = parse_injection_file(example_test_RegularCase.to_string()).unwrap();
        println!("{:?}", parsing_result);

        let expect = InjectionConfiguration::new(
            Some(SearchType::module_name),
            Some("something.exe".to_string()),
            vec![
                InjectionEntry::new( vec![0x90, 0x90], None, 1 ),
                InjectionEntry::new( vec![0x90, 0x90, 0x90, 0x90], Some((0, 2)), 1 ),
                InjectionEntry::new( vec![0xAA, 0xAA, 0x90, 0x90], Some((0, 2)), 1 ),
            ]
        );
        assert_eq!(expect, parsing_result);
    }

    const example_test_RegularCase_ModuleNameWithSpaces: &str = "    
search_type = module
module_name = A name with spaces.exe

instruction = 0x9090, matches_allowed = 1 // Search and replace the entirety of the instruction
instruction = 0x90909090, range = 0:2, matches_allowed = 1 // Search for this instruction, but only replace a range (start position:length in bytes)
instruction = 0xAAAA9090, range = 0:2, matches_allowed = 1 // Replace the first 2 bytes (\"0xAAAA\")

    ";

    #[test]
    fn InjectionFileParsing_ModuleNameWithSpaces()
    {
        let parsing_result = parse_injection_file(example_test_RegularCase_ModuleNameWithSpaces.to_string()).unwrap();
        println!("{:?}", parsing_result);

        let expect = InjectionConfiguration::new(
            Some(SearchType::module_name),
            Some("A name with spaces.exe".to_string()),
            vec![
                InjectionEntry::new( vec![0x90, 0x90], None, 1 ),
                InjectionEntry::new( vec![0x90, 0x90, 0x90, 0x90], Some((0, 2)), 1 ),
                InjectionEntry::new( vec![0xAA, 0xAA, 0x90, 0x90], Some((0, 2)), 1 ),
            ]
        );
        assert_eq!(expect, parsing_result);
    }

    #[test]
    fn InjectionFileParsing_RegularCase_ExeMemory()
    {
        let parsing_result = parse_injection_file(example_test_RegularCase_ExeMemory.to_string()).unwrap();
        println!("{:?}", parsing_result);

        let expect = InjectionConfiguration::new(
            Some(SearchType::exe_memory),
            None,
            vec![
                InjectionEntry::new( vec![0x90, 0x90], None, 1 ),
                InjectionEntry::new( vec![0x90, 0x90, 0x90, 0x90], Some((0, 2)), 1 ),
                InjectionEntry::new( vec![0xAA, 0xAA, 0x90, 0x90], Some((0, 2)), 1 ),
            ]
        );
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_UnknownSearchType: &str = "        
search_type = not_a_valid_search_type

instruction = 0xAAAA9090, range = 0:2, matches_allowed = 1 
    ";

    #[test]
    fn InjectionFileParsing_Error_UnknownSearchType()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_UnknownSearchType.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_UnknownSearchType;
        assert_eq!(expect, parsing_result);
    }


    const example_test_ValidationError_NoSearch: &str = "        
instruction = 0xAAAA9090, range = 0:2, matches_allowed = 1 
    ";

    #[test]
    fn InjectionFileParsing_Error_NoSearchType()
    {
        let parsing_result = parse_injection_file(example_test_ValidationError_NoSearch.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::ValidationError_NoSearchType;
        assert_eq!(expect, parsing_result);
    }

    const example_test_ValidationError_NoModuleName: &str = "   
search_type = module
instruction = 0xAAAA9090, range = 0:2, matches_allowed = 1 
    ";

    #[test]
    fn InjectionFileParsing_Error_NoModuleName()
    {
        let parsing_result = parse_injection_file(example_test_ValidationError_NoModuleName.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::ValidationError_NoModuleName;
        assert_eq!(expect, parsing_result);
    }

    const example_test_ValidationError_NoInstructions: &str = "   
search_type = module
module_name = hello.exe
    ";

    #[test]
    fn InjectionFileParsing_Error_NoInstructions()
    {
        let parsing_result = parse_injection_file(example_test_ValidationError_NoInstructions.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::ValidationError_NoInstructions;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_UnknownFiled: &str = "   
search_type = module
module_name = hello.exe
not_a_valid_field = 10
    ";

    #[test]
    fn InjectionFileParsing_Error_UnknownFiled()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_UnknownFiled.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_UnknownFiled;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_MissingKeyValuePair: &str = "   
search_type = module
module_name = hello.exe
instruction
    ";

    #[test]
    fn InjectionFileParsing_Error_MissingKeyValuePair()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_MissingKeyValuePair.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_MissingKeyValuePair;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_InvalidHex: &str = "   
search_type = module
module_name = hello.exe
instruction = 0xZZZZ, matches_allowed = 1 
    ";

    #[test]
    fn InjectionFileParsing_Error_InvalidHex()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_InvalidHex.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_InvalidHexInstruction;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_NotAHex: &str = "   
search_type = module
module_name = hello.exe
instruction = this_is_not_a_hex_value, matches_allowed = 1 
    ";

    #[test]
    fn InjectionFileParsing_Error_NotAHex()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_NotAHex.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_NotAHex;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_OddHex: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x909, matches_allowed = 1 
    ";

    #[test]
    fn InjectionFileParsing_Error_OddHex()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_OddHex.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_OddInstruction;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_InvalidRange: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, matches_allowed = 1, range = 0:0
    ";

    #[test]
    fn InjectionFileParsing_Error_InvalidRange()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_InvalidRange.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_InvalidRange_negative_start: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, matches_allowed = 1, range = -1:10
    ";

    #[test]
    fn InjectionFileParsing_Error_InvalidRange_Negative_Start()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_InvalidRange_negative_start.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_InvalidRange_negative_end: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, matches_allowed = 1, range = 1:-10
    ";

    #[test]
    fn InjectionFileParsing_Error_InvalidRange_Negative_End()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_InvalidRange_negative_end.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_InvalidRange_missing_value_pair: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, matches_allowed = 1, range = 99
    ";

    #[test]
    fn InjectionFileParsing_Error_InvalidRange_MissingPair()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_InvalidRange_missing_value_pair.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_InvalidRange_MissingPair;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_InvalidRange_letters: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, matches_allowed = 1, range = AA:AA
    ";

    #[test]
    fn InjectionFileParsing_Error_InvalidRange_Letters()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_InvalidRange_letters.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_InvalidRangeValue;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_InvalidParameter: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, matches_allowed = A, range = 0:10
    ";

    #[test]
    fn InjectionFileParsing_Error_InvalidParameter()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_InvalidParameter.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_InvalidParameter;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_EmptyMatches: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, range = 0:10
    ";

    #[test]
    fn InjectionFileParsing_Error_EmptyMatches()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_EmptyMatches.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_EmptyMatches;
        assert_eq!(expect, parsing_result);
    }

    // Duplicate input

    const example_test_EntryParsingError_Duplicate_SearchType: &str = "   
search_type = module
search_type = module
module_name = hello.exe
instruction = 0x9090, range = 0:10
    ";

    #[test]
    fn InjectionFileParsing_Error_Duplicate_SearchType()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_Duplicate_SearchType.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_Duplicates;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_Duplicate_ModuleName: &str = "   
search_type = module
module_name = hello.exe
module_name = hello.exe
instruction = 0x9090, range = 0:10
    ";

    #[test]
    fn InjectionFileParsing_Error_Duplicate_ModuleName()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_Duplicate_ModuleName.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_Duplicates;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_Duplicate_Instruction: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, instruction = 0xAAAA, range = 0:10
    ";

    #[test]
    fn InjectionFileParsing_Error_Duplicate_Instruction()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_Duplicate_Instruction.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_Duplicates;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_Duplicate_Matches: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, matches_allowed = 2, matches_allowed = 3, range = 0:10
    ";

    #[test]
    fn InjectionFileParsing_Error_Duplicate_Matches()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_Duplicate_Matches.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_Duplicates;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_Duplicate_Range: &str = "   
search_type = module
module_name = hello.exe
instruction = 0x9090, matches_allowed = 2, range = 0:10, range = 0:13
    ";

    #[test]
    fn InjectionFileParsing_Error_Duplicate_Range()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_Duplicate_Range.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_Duplicates;
        assert_eq!(expect, parsing_result);
    }

    const example_test_EntryParsingError_Mixing: &str = "   
search_type = module
instruction = 0x9090, matches_allowed = 2, range = 0:10, module_name = hello.exe
    ";

    #[test]
    fn InjectionFileParsing_Error_Mixing()
    {
        let parsing_result = parse_injection_file(example_test_EntryParsingError_Mixing.to_string()).unwrap_err();
        println!("{:?}", parsing_result);

        let expect = InjectionFileParsingErrors::EntryParsingError_MixingEntryTypes;
        assert_eq!(expect, parsing_result);
    }
}