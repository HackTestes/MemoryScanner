use crate::Configuration::*;
use crate::SearchEngines;
use crate::Matches;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum CommandParsingError
{
    InvalidAction,
    InvalidOption,
    InvalidParameter,
    MissingParameter,
    InvalidUnitOfMeasurement,
    InvalidTargetType,
    InvalidTargetValue,
    InvalidEngine,
    InsufficientNumOfThreads,
    InvalidCopyBufferSize,
    NotEnoughOperations,
    NoWriteAddress,
    NoTarget,
    InvalidDisplayStyle
}

// I am using modules to better organize the arguments
// It works similarly to C++'s namespaces
mod Options
{
    pub mod Help
    {
        pub const short_option: &str = "-h";
        pub const long_option: &str = "--help";
        pub const description: &str = "Displays help text";
    }

    // Search options

    // Thread result's storage
    pub mod ThreadStorage
    {
        pub const short_option: &str = "-ts";
        pub const long_option: &str = "--thread-storage";
        pub const description: &str = "Controls how many results a thread can initally store (higher values can reduce the amount of memory allocations)";
        pub const params: &[&str] = &["<NUM_OF_RESULTS_PER_THREAD>"];
    }

    // How many threads to use
    pub mod Threads
    {
        pub const short_option: &str = "-th";
        pub const long_option: &str = "--threads";
        pub const description: &str = "Controls the amount of threads used to perform the search";
        pub const params: &[&str] = &["<NUM_OF_THREADS>"];
    }

    // Buffer size used to store a copy of the process
    pub mod CopyBufferSize
    {
        pub const short_option: &str = "-b";
        pub const long_option: &str = "--buffer-size";
        pub const description: &str = "Controls how much memory the search can use while copying from the target (small values may be unable to search and higher values can increase speed). Example: 1 KiB, 10 MiB, 100 GiB";
        pub const params: &[&str] = &["<BUFFER_SIZE>", "<UNIT_OF_MEASUREMENT>"];
    }


    // What kind of type are we searching for?
    pub mod TargetType
    {
        pub const short_option: &str = "-ty";
        pub const long_option: &str = "--target-type";
        pub const description: &str = "Controls the data type of the target. Valid types: u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64";
        pub const params: &[&str] = &["<TARGET_TYPE>"];
    }


    pub mod TargetValue
    {
        pub const short_option: &str = "-t";
        pub const long_option: &str = "--target";
        pub const description: &str = "Value of the target (this is valid for exact matches and write operations)";
        pub const params: &[&str] = &["<TARGET_VALUE>"];
    }


    pub mod TargetOperations
    {
        pub const short_option: &str = "-to";
        pub const long_option: &str = "--target-operations";
        pub const description: &str = "Inputs a search operation, which consists of a comparison action and a target value pair: <CMP_OP> <TARGET>. Targets must be values. Valid comparison operations: ==, !=, >, >=, <, <=. Note: you can repeat this option to create a range";
        pub const params: &[&str] = &["<CMP_OPERATION>", "<TARGET_VALUE>"];
    }

    pub mod Filter
    {
        pub const short_option: &str = "-F";
        pub const long_option: &str = "--filter";
        pub const description: &str = "Filters the previous search (the default behaviour is to simply start a new search) in order to pinpoint the memory position";
    }

    pub mod Engine
    {
        pub const short_option: &str = "-e";
        pub const long_option: &str = "--engine";
        pub const description: &str = "Selects the search engine that will be used, please check for the documentation for more details about each engine. The default one is the comparator. Valid options: comparator, exact*, exact_memchr*, comparator_simd* (*: not yet supported)";
        pub const params: &[&str] = &["<ENGINE_NAME>"];
    }

    // Display options

    pub mod DisplayStyle
    {
        pub const short_option: &str = "-ds";
        pub const long_option: &str = "--display-style";
        pub const description: &str = "Selects the format that will be used to display absolute address of each match. It uses hexadecimal by default. Valid styles: dec (decimal), hex";
        pub const params: &[&str] = &["<DISPLAY_STYLE>"];
    }

    // Result saving options

    pub mod RestoreSpecificEntry
    {
        pub const short_option: &str = "-re";
        pub const long_option: &str = "--restore-entry";
        pub const description: &str = "Restores a specific entry from the queue by passing the queue index (Note: it starts at zero)";
        pub const params: &[&str] = &["<ENTRY_INDEX>"];
    }

    pub mod RemoveAllEntries
    {
        // Does not have a short option
        pub const long_option: &str = "--all";
        pub const description: &str = "Removes all results from the saved queue";
    }

    // Write options

    pub mod Freeze
    {
        pub const short_option: &str = "-fz";
        pub const long_option: &str = "--freeze";
        pub const description: &str = "Freeze a value in memory by repeatedly writing the same target (Note: you need a absolute address specified)";
    }

    pub mod WriteAbsAddr
    {
        pub const short_option: &str = "-aa";
        pub const long_option: &str = "--absolute-address";
        pub const description: &str = "The absolute address in the target that will get something written to";
        pub const params: &[&str] = &["<ABSOLUTE_ADDRESS_DECIMAL>"];
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum ActionsEnum
{
    Help,
    Search,
    Write,
    Save,
    Restore,
    Remove
}

mod Actions
{
    pub mod Help
    {
        pub const text: &str = "help";
        pub const description: &str = "Does the same thing as the help option (displays help text). This is simply to help new users";
    }

    pub mod Search
    {
        pub const text: &str = "search";
        pub const description: &str = "Starts a new search";
    }

    pub mod Display
    {
        pub const text: &str = "display";
        pub const description: &str = "Shows the absolute memory addresses of the matches in the target process";
    }

    pub mod Write
    {
        pub const text: &str = "write";
        pub const description: &str = "Write a value into the memory of the target value (it defaults to writing it once)";
    }

    pub mod Save
    {
        pub const text: &str = "save";
        pub const description: &str = "Saves the current search results for later use (note that this can increase the memory overhead)";
    }

    pub mod Restore
    {
        pub const text: &str = "restore";
        pub const description: &str = "Restores a previous saved result (it does not remove the entry). By default, it restores the last saved entry";
    }

    pub mod Remove
    {
        pub const text: &str = "remove";
        pub const description: &str = "Removes a saved result from the stack or all of them. By default, it removes only the last entry";
    }
}

fn argument_parsing(command: String) -> Result<Config, CommandParsingError>
{
    // Create the default configuration to be later modified
    let mut configuration = Config::new();

    // Split the command to emulate a normal argument from command line
    let mut command_list: Vec<_> = command.split(" ").collect();

    // Get actions
    // Actions should always be the first item
    let action: String = command_list[0].to_string();
    command_list.remove(0);

    // Match the action and update the configuration
    configuration.action = match action.as_str()
    {
        Actions::Help::text    => ActionsEnum::Help,
        Actions::Search::text  => ActionsEnum::Search,
        Actions::Write::text   => ActionsEnum::Write,
        Actions::Save::text    => ActionsEnum::Save,
        Actions::Restore::text => ActionsEnum::Restore,
        Actions::Remove::text  => ActionsEnum::Remove,
        _ => {
            eprintln!("Invalid action: {}", action);
            return Err(CommandParsingError::InvalidAction);
        },
    };

    // If we have the help action, we don't need to validate anything else
    if configuration.action == ActionsEnum::Help
    {
        println!("HELP PLACEHOLDER");
        configuration.help = true;
        return Ok(configuration);
    }

    // Get options
    // Everything else is an option or an option input parameter
    // The while loops allows me to control the loop index
    let mut opt_index = 0;
    while opt_index < command_list.len()
    {
        let current_option = command_list[opt_index];

        match current_option
        {
            Options::Help::short_option | Options::Help::long_option =>
            {
                println!("HELP PLACEHOLDER");
                configuration.help = true;

                // Early break, help doesn't need anything else
                break;
            },

            Options::ThreadStorage::short_option | Options::ThreadStorage::long_option =>
            {
                // Validate size
                if opt_index+Options::ThreadStorage::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let number_results_per_thread = command_list[opt_index+1].parse::<usize>();

                if number_results_per_thread.is_err()
                {
                    eprintln!("Invalid parameter of option {}: {}", current_option, command_list[opt_index+1]);
                    return Err(CommandParsingError::InvalidParameter);
                }

                configuration.thread_storage = number_results_per_thread.unwrap();
                opt_index += Options::ThreadStorage::params.len(); // Jumps the input param
            },

            Options::Threads::short_option | Options::Threads::long_option =>
            {
                // Validate size
                if opt_index+Options::Threads::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let num_threads_r = command_list[opt_index+1].parse::<u64>();

                if num_threads_r.is_err()
                {
                    eprintln!("Invalid parameter of option {}: {}", current_option, command_list[opt_index+1]);
                    return Err(CommandParsingError::InvalidParameter);
                }

                let num_threads = num_threads_r.unwrap();

                if num_threads < 1
                {
                    eprintln!("You must pass at least 1 thread to perform the search");
                    return Err(CommandParsingError::InsufficientNumOfThreads);
                }

                configuration.num_threads = num_threads;
                opt_index += Options::Threads::params.len(); // Jumps the input param
            },

            Options::CopyBufferSize::short_option | Options::CopyBufferSize::long_option =>
            {
                // Validate size
                if opt_index+Options::CopyBufferSize::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let size_number_r = command_list[opt_index+1].parse::<usize>();

                if size_number_r.is_err()
                {
                    eprintln!("Invalid parameter of option {}: {}", current_option, command_list[opt_index+1]);
                    return Err(CommandParsingError::InvalidParameter);
                }

                let size_number = size_number_r.unwrap();

                if size_number < 1
                {
                    eprintln!("The size of the copy buffer needs to be higher than 0");
                    return Err(CommandParsingError::InvalidCopyBufferSize);
                }

                let unit_measurement = command_list[opt_index+2];
                let mut copy_buffer_size_bytes: usize = 0;

                copy_buffer_size_bytes = match unit_measurement
                {
                    "KiB" => size_number*1024,
                    "MiB" => size_number*1024*1024,
                    "GiB" => size_number*1024*1024*1024,
                    _ => {
                        eprintln!("Invalid unit OF measurement: {}", unit_measurement);
                        return Err(CommandParsingError::InvalidUnitOfMeasurement);
                    }
                };

                configuration.copy_buffer_size = copy_buffer_size_bytes;
                opt_index += Options::CopyBufferSize::params.len(); // Jumps the input param
            },

            Options::TargetType::short_option | Options::TargetType::long_option =>
            {
                // Validate size
                if opt_index+Options::TargetType::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let target_type_input = command_list[opt_index+1];

                let target_type_enum: TargetType = match target_type_input
                {
                    "u8" => TargetType::u8,
                    "u16" => TargetType::u16,
                    "u32" => TargetType::u32,
                    "u64" => TargetType::u64,
                    "u128" => TargetType::u128,
                    "i8" => TargetType::i8,
                    "i16" => TargetType::i16,
                    "i32" => TargetType::i32,
                    "i64" => TargetType::i64,
                    "i128" => TargetType::i128,
                    "f32" => TargetType::f32,
                    "f64" => TargetType::f64,
                    _ => {
                        eprintln!("Invalid target type of option {}: {}",current_option, target_type_input);
                        return Err(CommandParsingError::InvalidTargetType);
                    }
                };

                configuration.target_type = target_type_enum;
                opt_index += Options::TargetType::params.len(); // Jumps the input param
            },

            Options::TargetValue::short_option | Options::TargetValue::long_option =>
            {
                // Validate size
                if opt_index+Options::TargetValue::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let target_value = command_list[opt_index+1];

                // I will defer the checking to the end, so I can validate if the value can be parsed to the correct target type

                configuration.target = Some(target_value.to_string());
                opt_index += Options::TargetValue::params.len(); // Jumps the input param
            },

            Options::TargetOperations::short_option | Options::TargetOperations::long_option =>
            {
                // Validate size
                if opt_index+Options::TargetOperations::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let cmp_op = command_list[opt_index+1];

                let cmp_op_enum: SearchEngines::ComparisonOperation = match cmp_op
                {
                    "==" => SearchEngines::ComparisonOperation::Equal,
                    "!=" => SearchEngines::ComparisonOperation::Unequal,
                    ">" => SearchEngines::ComparisonOperation::Greater,
                    ">=" => SearchEngines::ComparisonOperation::GreaterOrEqual,
                    "<" => SearchEngines::ComparisonOperation::Less,
                    "<=" => SearchEngines::ComparisonOperation::LessOrEqual,
                    _ => {
                        eprintln!("Invalid comparison operation of option {}: {}",current_option, cmp_op);
                        return Err(CommandParsingError::InvalidTargetType);
                    }
                };

                let op_target = command_list[opt_index+2].to_string();

                // I will defer the checking to the end, so I can validate if the value can be parsed to the correct target type

                let operation_pair: (SearchEngines::ComparisonOperation, String) = (cmp_op_enum, op_target);

                configuration.operations.push(operation_pair);
                opt_index += Options::TargetOperations::params.len(); // Jumps the input param
            },

            Options::Filter::short_option | Options::Filter::long_option =>
            {
                configuration.filter = true;
            },

            Options::Engine::short_option | Options::Engine::long_option =>
            {
                // Validate size
                if opt_index+Options::Engine::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let engine_name = command_list[opt_index+1];

                let engine_name_enum: SearchEngines::Engines = match engine_name
                {
                    "comparator" => SearchEngines::Engines::comparator,

                    // WARNING: NOT SUPPORTED YET
                    "exact" => SearchEngines::Engines::exact,
                    _ => {
                        eprintln!("Invalid engine: {}", engine_name);
                        return Err(CommandParsingError::InvalidEngine);
                    }
                };

                configuration.engine = engine_name_enum;
                opt_index += Options::Engine::params.len(); // Jumps the input param
            },

            Options::DisplayStyle::short_option | Options::DisplayStyle::long_option =>
            {
                // Validate size
                if opt_index+Options::DisplayStyle::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let display_style = command_list[opt_index+1];

                let display_style_enum: Matches::MatchDisplayStyle = match display_style
                {
                    "hex" => Matches::MatchDisplayStyle::Hex,
                    "dec" => Matches::MatchDisplayStyle::Decimal,
                    _ => {
                        eprintln!("Invalid display style: {}", display_style);
                        return Err(CommandParsingError::InvalidDisplayStyle);
                    }
                };

                configuration.display_style = display_style_enum;
                opt_index += Options::DisplayStyle::params.len(); // Jumps the input param
            },

            Options::RestoreSpecificEntry::short_option | Options::RestoreSpecificEntry::long_option =>
            {
                // Validate size
                if opt_index+Options::RestoreSpecificEntry::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let entry = command_list[opt_index+1].parse::<usize>();

                if entry.is_err()
                {
                    eprintln!("Invalid parameter of option {}: {}", current_option, command_list[opt_index+1]);
                    return Err(CommandParsingError::InvalidParameter);
                }

                configuration.restore_entry = Some(entry.unwrap());
                opt_index += Options::RestoreSpecificEntry::params.len(); // Jumps the input param
            },

            Options::RemoveAllEntries::long_option =>
            {
                configuration.remove_all_saved_entries = true;
            },

            Options::Freeze::short_option | Options::Freeze::long_option =>
            {
                configuration.freeze = true;
            },

            Options::WriteAbsAddr::short_option | Options::WriteAbsAddr::long_option =>
            {
                // Validate size
                if opt_index+Options::WriteAbsAddr::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let abs_addr = command_list[opt_index+1].parse::<usize>();

                if abs_addr.is_err()
                {
                    eprintln!("Invalid parameter of option {}: {}", current_option, command_list[opt_index+1]);
                    return Err(CommandParsingError::InvalidParameter);
                }

                configuration.write_abs_addr = Some(abs_addr.unwrap());
                opt_index += Options::WriteAbsAddr::params.len(); // Jumps the input param
            },

            _ =>
            {
                eprintln!("Invalid option: {}", current_option);
                return Err(CommandParsingError::InvalidOption);
            }

        }

        // Go to the next loop or we get an infinite loop
        opt_index += 1;
    }

    // Validate the configuration
    // Here we check things after all of the parsing is done

    // Does the comparator engine have no operation?
    if (configuration.action == ActionsEnum::Search) && (configuration.engine == SearchEngines::Engines::comparator)
    {
        if configuration.operations.len() == 0
        {
            return Err(CommandParsingError::NotEnoughOperations);
        }

        // Can we convert the target in operations?
        // We can only do this check is operations actually exist
        for op_pair in configuration.operations.iter()
        {
            let target_string = op_pair.1.clone();

            // We do the verification based on the configured target type
            let is_it_valid = match configuration.target_type
            {
                TargetType::u8 => target_string.parse::<u8>().is_ok(),
                TargetType::u16 => target_string.parse::<u16>().is_ok(),
                TargetType::u32 => target_string.parse::<u32>().is_ok(),
                TargetType::u64 => target_string.parse::<u64>().is_ok(),
                TargetType::u128 => target_string.parse::<u128>().is_ok(),
                TargetType::i8 => target_string.parse::<i8>().is_ok(),
                TargetType::i16 => target_string.parse::<i16>().is_ok(),
                TargetType::i32 => target_string.parse::<i32>().is_ok(),
                TargetType::i64 => target_string.parse::<i64>().is_ok(),
                TargetType::i128 => target_string.parse::<i128>().is_ok(),
                TargetType::f32 => target_string.parse::<f32>().is_ok(),
                TargetType::f64 => target_string.parse::<f64>().is_ok(),
                _ => {
                    eprintln!("Invalid target value in operation: {:?}", op_pair);
                    return Err(CommandParsingError::InvalidTargetType);
                }
            };

            if is_it_valid == false
            {
                return Err(CommandParsingError::InvalidTargetValue);
            }
        }
    }

    // Does the other engines have a target?
    // For now we only have the comparator. so there is not need to check anything here

    // Can we convert the target?
    if configuration.target != None
    {
        let target_string = configuration.target.clone().unwrap();

        // We do the verification based on the configured target type
        let is_it_valid = match configuration.target_type
        {
            TargetType::u8 => target_string.parse::<u8>().is_ok(),
            TargetType::u16 => target_string.parse::<u16>().is_ok(),
            TargetType::u32 => target_string.parse::<u32>().is_ok(),
            TargetType::u64 => target_string.parse::<u64>().is_ok(),
            TargetType::u128 => target_string.parse::<u128>().is_ok(),
            TargetType::i8 => target_string.parse::<i8>().is_ok(),
            TargetType::i16 => target_string.parse::<i16>().is_ok(),
            TargetType::i32 => target_string.parse::<i32>().is_ok(),
            TargetType::i64 => target_string.parse::<i64>().is_ok(),
            TargetType::i128 => target_string.parse::<i128>().is_ok(),
            TargetType::f32 => target_string.parse::<f32>().is_ok(),
            TargetType::f64 => target_string.parse::<f64>().is_ok(),
            _ => {
                eprintln!("Invalid target value in operation: {}", target_string);
                return Err(CommandParsingError::InvalidTargetType);
            }
        };

        if is_it_valid == false
        {
            return Err(CommandParsingError::InvalidTargetValue);
        }
    }

    // Does write have a target address and an actual value?
    if configuration.action == ActionsEnum::Write
    {
        if configuration.write_abs_addr == None
        {
            return Err(CommandParsingError::NoWriteAddress)
        }

        if configuration.target == None
        {
            return Err(CommandParsingError::NoTarget)
        }
    }

    return Ok(configuration);
}

#[cfg(test)]
mod tests
{
    use crate::CLIFrontEnd::*;

    // Action tests

    #[test]
    fn CLITest_Action_Help()
    {
        let parsing_result = argument_parsing("help".to_string());
        assert_eq!(parsing_result.unwrap().help, true);
    }

    #[test]
    fn CLITest_Action_Search()
    {
        let parsing_result = argument_parsing("search -to == 10".to_string());

        let parsed_config = parsing_result.unwrap();
        assert_eq!(parsed_config.action, ActionsEnum::Search);
    }

    #[test]
    fn CLITest_Action_SearchFail()
    {
        let parsing_result = argument_parsing("search".to_string());

        assert_eq!(parsing_result, Err(CommandParsingError::NotEnoughOperations));
    }

    #[test]
    fn CLITest_Action_Write()
    {
        let parsing_result = argument_parsing("write -aa 1000 -t 10".to_string());

        let parsed_config = parsing_result.unwrap();
        assert_eq!(parsed_config.action, ActionsEnum::Write);
    }

    #[test]
    fn CLITest_Action_WriteFail()
    {
        let parsing_result = argument_parsing("write".to_string());

        assert_eq!(parsing_result, Err(CommandParsingError::NoWriteAddress));
    }

    #[test]
    fn CLITest_Action_WriteFail_NoTarget()
    {
        let parsing_result = argument_parsing("write -aa 1000".to_string());

        assert_eq!(parsing_result, Err(CommandParsingError::NoTarget));
    }

    #[test]
    fn CLITest_Action_Save()
    {
        let parsing_result = argument_parsing("save".to_string());

        let parsed_config = parsing_result.unwrap();
        assert_eq!(parsed_config.action, ActionsEnum::Save);
    }

    #[test]
    fn CLITest_Action_Restore()
    {
        let parsing_result = argument_parsing("restore".to_string());

        let parsed_config = parsing_result.unwrap();
        assert_eq!(parsed_config.action, ActionsEnum::Restore);
    }

    #[test]
    fn CLITest_Action_Remove()
    {
        let parsing_result = argument_parsing("remove".to_string());

        let parsed_config = parsing_result.unwrap();
        assert_eq!(parsed_config.action, ActionsEnum::Remove);
    }

    // Option tests
    #[test]
    fn CLITest_Option_ThreadStorage()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -ts 10".to_string()).unwrap().thread_storage,
            10);

        assert_eq!(
            argument_parsing("search -to == 10 --thread-storage 100".to_string()).unwrap().thread_storage,
            100);
    }

    #[test]
    fn CLITest_Option_ThreadNum()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -th 10".to_string()).unwrap().num_threads,
            10);

        assert_eq!(
            argument_parsing("search -to == 10 --threads 100".to_string()).unwrap().num_threads,
            100);
    }

    #[test]
    fn CLITest_Option_ThreadNumFail()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -th 0".to_string()),
            Err(CommandParsingError::InsufficientNumOfThreads));
    }

    #[test]
    fn CLITest_Option_CopyBufferSize()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -b 1 KiB".to_string()).unwrap().copy_buffer_size,
            1024);

        assert_eq!(
            argument_parsing("search -to == 10 --buffer-size 1 MiB".to_string()).unwrap().copy_buffer_size,
            1024*1024);

        assert_eq!(
            argument_parsing("search -to == 10 --buffer-size 1 GiB".to_string()).unwrap().copy_buffer_size,
            1024*1024*1024);
    }

    #[test]
    fn CLITest_Option_CopyBufferSizeFail()
    {
        assert_eq!(
            argument_parsing("search -to == 10 --buffer-size 0 GiB".to_string()),
            Err(CommandParsingError::InvalidCopyBufferSize));
    }

    // TODO: Test the values as well
    #[test]
    fn CLITest_Option_TargetType()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -ty u8".to_string()).unwrap().target_type,
            TargetType::u8);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type u16".to_string()).unwrap().target_type,
            TargetType::u16);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type u32".to_string()).unwrap().target_type,
            TargetType::u32);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type u64".to_string()).unwrap().target_type,
            TargetType::u64);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type i8".to_string()).unwrap().target_type,
            TargetType::i8);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type i16".to_string()).unwrap().target_type,
            TargetType::i16);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type i32".to_string()).unwrap().target_type,
            TargetType::i32);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type i64".to_string()).unwrap().target_type,
            TargetType::i64);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type f32".to_string()).unwrap().target_type,
            TargetType::f32);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type f64".to_string()).unwrap().target_type,
            TargetType::f64);
    }

    #[test]
    fn CLITest_Option_TargetTypeFail()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -ty u7".to_string()),
            Err(CommandParsingError::InvalidTargetType));
    }

    #[test]
    fn CLITest_Option_Target()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -t 124".to_string()).unwrap().target,
            Some("124".to_string()));
    }

    #[test]
    fn CLITest_Option_Target_InvalidInputs()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -t a".to_string()),
            Err(CommandParsingError::InvalidTargetValue));

        assert_eq!(
            argument_parsing("search -to == 10 -ty u8 -t 1024".to_string()),
            Err(CommandParsingError::InvalidTargetValue));
    }

    // TODO: Add tests for operations


    #[test]
    fn CLITest_Option_Filter()
    {
        // Default
        assert_eq!(
            argument_parsing("search -to == 10".to_string()).unwrap().filter,
            false);

        assert_eq!(
            argument_parsing("search -to == 10 -F".to_string()).unwrap().filter,
            true);
    }

    #[test]
    fn CLITest_Option_Engine()
    {
        // Default
        assert_eq!(
            argument_parsing("search -to == 10".to_string()).unwrap().engine,
            SearchEngines::Engines::comparator);

        assert_eq!(
            argument_parsing("search -to == 10 -e exact".to_string()).unwrap().engine,
            SearchEngines::Engines::exact);

        assert_eq!(
            argument_parsing("search -to == 10 --engine exact".to_string()).unwrap().engine,
            SearchEngines::Engines::exact);
    }

    #[test]
    fn CLITest_Option_EngineFail()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -e engine-that-does-not-exist".to_string()),
            Err(CommandParsingError::InvalidEngine));
    }

    #[test]
    fn CLITest_Option_DisplayStyle()
    {
        // Default
        assert_eq!(
            argument_parsing("search -to == 10".to_string()).unwrap().display_style,
            Matches::MatchDisplayStyle::Hex);

        assert_eq!(
            argument_parsing("search -to == 10 -ds dec".to_string()).unwrap().display_style,
            Matches::MatchDisplayStyle::Decimal);

        assert_eq!(
            argument_parsing("search -to == 10 --display-style dec".to_string()).unwrap().display_style,
            Matches::MatchDisplayStyle::Decimal);
    }

    #[test]
    fn CLITest_Option_DisplayStyleFail()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -ds invalid-style".to_string()),
            Err(CommandParsingError::InvalidDisplayStyle));
    }

    #[test]
    fn CLITest_Option_EntryRestore()
    {
        // Default
        assert_eq!(
            argument_parsing("search -to == 10".to_string()).unwrap().restore_entry,
            None);

        assert_eq!(
            argument_parsing("search -to == 10 -re 5".to_string()).unwrap().restore_entry,
            Some(5));

        assert_eq!(
            argument_parsing("search -to == 10 --restore-entry 5".to_string()).unwrap().restore_entry,
            Some(5));
    }

    #[test]
    fn CLITest_Option_EntryRestoreFail()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -re aaa".to_string()),
            Err(CommandParsingError::InvalidParameter));
    }

    #[test]
    fn CLITest_Option_RemoveAllEntries()
    {
        // Default
        assert_eq!(
            argument_parsing("remove".to_string()).unwrap().remove_all_saved_entries,
            false);

        assert_eq!(
            argument_parsing("remove --all".to_string()).unwrap().remove_all_saved_entries,
            true);
    }

    #[test]
    fn CLITest_Option_Freeze()
    {
        // Default
        assert_eq!(
            argument_parsing("write -aa 1000 -t 10".to_string()).unwrap().freeze,
            false);

        assert_eq!(
            argument_parsing("write -aa 1000 -t 10 -fz".to_string()).unwrap().freeze,
            true);

        assert_eq!(
            argument_parsing("write -aa 1000 -t 10 --freeze".to_string()).unwrap().freeze,
            true);
    }

    #[test]
    fn CLITest_Option_WriteAbsAddress()
    {
        assert_eq!(
            argument_parsing("write -aa 1000 -t 10".to_string()).unwrap().write_abs_addr,
            Some(1000));

        assert_eq!(
            argument_parsing("write --absolute-address 1500 -t 10".to_string()).unwrap().write_abs_addr,
            Some(1500));
    }

    #[test]
    fn CLITest_Option_WriteAbsAddressFail()
    {
        assert_eq!(
            argument_parsing("write -aa -100 -t 10".to_string()),
            Err(CommandParsingError::InvalidParameter));

        assert_eq!(
            argument_parsing("write -aa agate -t 10".to_string()),
            Err(CommandParsingError::InvalidParameter));
    }

}