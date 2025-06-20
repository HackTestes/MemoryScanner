use crate::Configuration::*;
use crate::SearchEngines;
use crate::Matches;
use crate::GenericOSInterface;
use crate::Tokenization;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum CommandParsingError
{
    InvalidCommand(Tokenization::TokenizationError),
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
    InvalidDisplayStyle,
    InvalidComparisonOperation,
    InvalidPagePermission,
    NoFilePath,
    InvalidFreezeInterval
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
        pub const description: &str = "Controls the data type of the target. Uses u8 by default. Valid types: u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64";
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
        pub const long_option: &str = "--target-operation";
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

    pub mod PagePermissionsAtLeast
    {
        pub const short_option: &str = "-pal";
        pub const long_option: &str = "--page-perms-at-least";
        pub const description: &str = "Selects the permissions the the page must at least have to searched, this means that 'read|write' will also get pages that have 'execute' as long as it can be read and written. Example: read, read|write (execute pages are also valid), read|write|execute";
        pub const params: &[&str] = &["<PAGE_PERMS[|...]>"];
    }

    pub mod PagePermissionsExact
    {
        pub const short_option: &str = "-pe";
        pub const long_option: &str = "--page-perms-exact";
        pub const description: &str = "Selects the permissions the the page have to searched, but it is an exact match. This means that 'read|write' excludes all 'execute' pages Example: read, read|write (execute pages are NOT valid), read|write|execute";
        pub const params: &[&str] = &["<PAGE_PERMS[|...]>"];
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

    pub mod FreezeInterval
    {
        pub const short_option: &str = "-fzi";
        pub const long_option: &str = "--freeze-interval";
        pub const description: &str = "Controls the interval in which the target is written to the observed process in miliseconds";
        pub const params: &[&str] = &["<FREEZE_INTERVAL_MS>"];
    }

    pub mod WriteAbsAddr
    {
        pub const short_option: &str = "-aa";
        pub const long_option: &str = "--absolute-address";
        pub const description: &str = "The absolute address in the target that will get something written to";
        pub const params: &[&str] = &["<ABSOLUTE_ADDRESS_DECIMAL>"];
    }

    pub mod File
    {
        pub const short_option: &str = "-f";
        pub const long_option: &str = "--file";
        pub const description: &str = "A file path that holds additional configuration. In the case of code injection, it holds intructions to replace. NOTE: paths must not caontain spaces";
        pub const params: &[&str] = &["<FILE_PATH>"];
    }

    pub mod DryRun
    {
        pub const short_option: &str = "-dr";
        pub const long_option: &str = "--dry-run";
        pub const description: &str = "Run the commands as normal but don't perform any change to the target. Only valid for code injection";
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Clone)]
pub enum ActionsEnum
{
    Help,
    Search,
    Display,
    Write,
    Save,
    Restore,
    Remove,
    Inject,
    Exit
}

mod Actions
{
    pub mod Help
    {
        pub const text: &str = "help";
        pub const description: &str = "Does the same thing as the help option (displays help text). This is simply to help new users";
    }

    pub mod Exit
    {
        pub const text: &str = "exit";
        pub const description: &str = "Exits from the command line, terminating the program";
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

    pub mod Inject
    {
        pub const text: &str = "inject";
        pub const description: &str = "Injects code into the attached process using instructions from a configuration file";
    }
}

pub fn print_CLI_help()
{
    let mut output: String = String::new();

    output.push_str("HELP OUPTPUT \n\tHere is how you can enter commands during the program's execution");

    output.push_str("\n\n");

    output.push_str("USAGE \n\tACTION [OPTIONS...]");

    output.push_str("\n\n");

    output.push_str("ACTIONS\n");
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Help::text, Actions::Help::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Exit::text, Actions::Exit::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Search::text, Actions::Search::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Write::text, Actions::Write::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Display::text, Actions::Display::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Save::text, Actions::Save::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Restore::text, Actions::Restore::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Remove::text, Actions::Remove::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Actions::Inject::text, Actions::Inject::description).as_str());

    output.push_str("OPTIONS\n");
    output.push_str(format!("\t{}, {} \n\t\t{}\n\n\n", Options::Help::long_option, Options::Help::short_option, Options::Help::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::Threads::long_option, Options::Threads::short_option, Options::Threads::params.join(" "), Options::Threads::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::ThreadStorage::long_option, Options::ThreadStorage::short_option, Options::ThreadStorage::params.join(" "), Options::ThreadStorage::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::CopyBufferSize::long_option, Options::CopyBufferSize::short_option, Options::CopyBufferSize::params.join(" "), Options::CopyBufferSize::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::TargetType::long_option, Options::TargetType::short_option, Options::TargetType::params.join(" "), Options::TargetType::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::TargetValue::long_option, Options::TargetValue::short_option, Options::TargetValue::params.join(" "), Options::TargetValue::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::TargetOperations::long_option, Options::TargetOperations::short_option, Options::TargetOperations::params.join(" "), Options::TargetOperations::description).as_str());
    output.push_str(format!("\t{}, {} \n\t\t{}\n\n\n", Options::Filter::long_option, Options::Filter::short_option, Options::Filter::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::Engine::long_option, Options::Engine::short_option, Options::Engine::params.join(" "), Options::Engine::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::PagePermissionsAtLeast::long_option, Options::PagePermissionsAtLeast::short_option, Options::PagePermissionsAtLeast::params.join(" "), Options::PagePermissionsAtLeast::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::PagePermissionsExact::long_option, Options::PagePermissionsExact::short_option, Options::PagePermissionsExact::params.join(" "), Options::PagePermissionsExact::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::DisplayStyle::long_option, Options::DisplayStyle::short_option, Options::DisplayStyle::params.join(" "), Options::DisplayStyle::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::RestoreSpecificEntry::long_option, Options::RestoreSpecificEntry::short_option, Options::RestoreSpecificEntry::params.join(" "), Options::RestoreSpecificEntry::description).as_str());
    output.push_str(format!("\t{} \n\t\t{}\n\n\n", Options::RemoveAllEntries::long_option, Options::RemoveAllEntries::description).as_str());
    output.push_str(format!("\t{}, {} \n\t\t{}\n\n\n", Options::Freeze::long_option, Options::Freeze::short_option, Options::Freeze::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::FreezeInterval::long_option, Options::FreezeInterval::short_option, Options::FreezeInterval::params.join(" "), Options::FreezeInterval::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::WriteAbsAddr::long_option, Options::WriteAbsAddr::short_option, Options::WriteAbsAddr::params.join(" "), Options::WriteAbsAddr::description).as_str());
    output.push_str(format!("\t{}, {} {} \n\t\t{}\n\n\n", Options::File::long_option, Options::File::short_option, Options::File::params.join(" "), Options::File::description).as_str());
    output.push_str(format!("\t{}, {} \n\t\t{}\n\n\n", Options::DryRun::long_option, Options::DryRun::short_option, Options::DryRun::description).as_str());


    println!("{}", output);
}

fn is_the_value_valid_for_type(target_string: &str, target_type: &TargetType) -> bool
{
    // We do the verification based on the configured target type
    return match target_type
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
    };
}

fn page_permissions_parse(page_permissions_string: &str) -> Result< GenericOSInterface::GenericPageProtections, CommandParsingError>
{
    let individual_permissions = page_permissions_string.split("|");

    let mut final_permission: GenericOSInterface::GenericPageProtections = GenericOSInterface::PageProtection_NoAccess;

    for perm in individual_permissions
    {
        match perm
        {
            "read" => final_permission |= GenericOSInterface::PageProtection_Read,
            "write" => final_permission |= GenericOSInterface::PageProtection_Write,
            "execute" => final_permission |= GenericOSInterface::PageProtection_Execute,
            _ => {
                return Err(CommandParsingError::InvalidPagePermission);
            }
        }
    }

    return Ok(final_permission);
}

pub fn argument_parsing(command: String) -> Result<Config, CommandParsingError>
{
    // Create the default configuration to be later modified
    let mut configuration = Config::new();

    // Tokenize the command to emulate a normal argument from command line
    let command_list_r = Tokenization::tokenize(command);

    let mut command_list: Vec<_> = match command_list_r
    {
        Ok(value) => value,
        Err(error) => return Err(CommandParsingError::InvalidCommand(error))
    };

    // Get actions
    // Actions should always be the first item
    let action: String = command_list[0].to_string();
    command_list.remove(0);

    // Match the action and update the configuration
    configuration.action = match action.as_str()
    {
        Actions::Help::text    => ActionsEnum::Help,
        Actions::Exit::text    => ActionsEnum::Exit,
        Actions::Display::text => ActionsEnum::Display,
        Actions::Search::text  => ActionsEnum::Search,
        Actions::Write::text   => ActionsEnum::Write,
        Actions::Save::text    => ActionsEnum::Save,
        Actions::Restore::text => ActionsEnum::Restore,
        Actions::Remove::text  => ActionsEnum::Remove,
        Actions::Inject::text  => ActionsEnum::Inject,
        _ => {
            eprintln!("Invalid action: {}", action);
            return Err(CommandParsingError::InvalidAction);
        },
    };

    // If we have the help action, we don't need to validate anything else
    if configuration.action == ActionsEnum::Help
    {
        print_CLI_help();
        configuration.help = true;
        return Ok(configuration);
    }

    // Get options
    // Everything else is an option or an option input parameter
    // The while loops allows me to control the loop index
    let mut opt_index = 0;
    while opt_index < command_list.len()
    {
        let current_option = command_list[opt_index].as_str();

        match current_option
        {
            Options::Help::short_option | Options::Help::long_option =>
            {
                print_CLI_help();
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

                let num_threads_r = command_list[opt_index+1].parse::<usize>();

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

                let unit_measurement = command_list[opt_index+2].as_str();

                let copy_buffer_size_bytes: usize = match unit_measurement
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

                let target_type_input = command_list[opt_index+1].as_str();

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

                let target_value = command_list[opt_index+1].as_str();

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

                let cmp_op = command_list[opt_index+1].as_str();

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
                        return Err(CommandParsingError::InvalidComparisonOperation);
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

                let engine_name = command_list[opt_index+1].as_str();

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

            Options::PagePermissionsAtLeast::short_option | Options::PagePermissionsAtLeast::long_option =>
            {
                // Validate size
                if opt_index+Options::PagePermissionsAtLeast::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let page_perms_r = page_permissions_parse(&command_list[opt_index+1]);

                if page_perms_r.is_err()
                {
                    eprintln!("Invalid page permission of option {}: {}", current_option, command_list[opt_index+1]);
                    return Err(CommandParsingError::InvalidPagePermission);
                }

                configuration.page_permissions_at_least = page_perms_r.unwrap();
                opt_index += Options::PagePermissionsAtLeast::params.len(); // Jumps the input param
            },

            Options::PagePermissionsExact::short_option | Options::PagePermissionsExact::long_option =>
            {
                // Validate size
                if opt_index+Options::PagePermissionsExact::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let page_perms_r = page_permissions_parse(&command_list[opt_index+1]);

                if page_perms_r.is_err()
                {
                    eprintln!("Invalid page permission of option {}: {}", current_option, command_list[opt_index+1]);
                    return Err(CommandParsingError::InvalidPagePermission);
                }

                configuration.page_permissions_exact = Some(page_perms_r.unwrap());
                opt_index += Options::PagePermissionsExact::params.len(); // Jumps the input param
            },

            Options::DisplayStyle::short_option | Options::DisplayStyle::long_option =>
            {
                // Validate size
                if opt_index+Options::DisplayStyle::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let display_style = command_list[opt_index+1].as_str();

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

            Options::FreezeInterval::short_option | Options::FreezeInterval::long_option =>
            {
                // Validate size
                if opt_index+Options::FreezeInterval::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let freeze_interval_r = command_list[opt_index+1].parse::<u64>();

                if freeze_interval_r.is_err()
                {
                    eprintln!("Invalid freeze interval: {}", command_list[opt_index+1]);
                    return Err(CommandParsingError::InvalidFreezeInterval);
                }

                configuration.freeze_interval_ms = freeze_interval_r.unwrap();
                opt_index += Options::FreezeInterval::params.len(); // Jumps the input param
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

            Options::File::short_option | Options::File::long_option =>
            {
                // Validate size
                if opt_index+Options::File::params.len() >= command_list.len()
                {
                    eprintln!("Missing parameter");
                    return Err(CommandParsingError::MissingParameter);
                }

                let file_path: String = command_list[opt_index+1].to_string();

                configuration.file_path = Some(file_path);
                opt_index += Options::File::params.len(); // Jumps the input param
            },

            Options::DryRun::short_option | Options::DryRun::long_option =>
            {
                configuration.dry_run = true;
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
            eprintln!("There are no comparator operations");
            return Err(CommandParsingError::NotEnoughOperations);
        }

        // Can we convert the target in operations?
        // We can only do this check is operations actually exist
        for op_pair in configuration.operations.iter()
        {
            let target_string = op_pair.1.clone();

            // We do the verification based on the configured target type
            let is_it_valid = is_the_value_valid_for_type(&target_string, &configuration.target_type);

            if is_it_valid == false
            {
                eprintln!("Invalid target value: {}", target_string);
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
        let is_it_valid = is_the_value_valid_for_type(&target_string, &configuration.target_type);

        if is_it_valid == false
        {
            eprintln!("Invalid target value: {}", target_string);
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

    // Does the inject action have a configuration file?
    if configuration.action == ActionsEnum::Inject
    {
        if configuration.file_path == None
        {
            return Err(CommandParsingError::NoFilePath)
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
        // Default
        assert_eq!(
            argument_parsing("exit".to_string()).unwrap().help,
            false);

        assert_eq!(
            argument_parsing("help".to_string()).unwrap().help,
            true);
    }

    #[test]
    fn CLITest_Action_Exit()
    {
        assert_eq!(
            argument_parsing("exit".to_string()).unwrap().action,
            ActionsEnum::Exit);
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
    fn CLITest_Action_Display()
    {
        assert_eq!(
            argument_parsing("display".to_string()).unwrap().action,
            ActionsEnum::Display);
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

    #[test]
    fn CLITest_Action_Inject()
    {
        assert_eq!(
            argument_parsing("inject -f c:\\windows\\file\\path".to_string()).unwrap().action,
            ActionsEnum::Inject);
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
    fn CLITest_Option_TargetType_FloatParsing()
    {
        assert_eq!(
            argument_parsing("search -to == 10 --target-type f32 -t 10.5".to_string()).unwrap().target_type,
            TargetType::f32);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type f64 -t 0.005".to_string()).unwrap().target_type,
            TargetType::f64);

        // You must use "." insted of ","
        assert_eq!(
            argument_parsing("search -to == 10 --target-type f64 -t 0,005".to_string()),
            Err(CommandParsingError::InvalidTargetValue));
    }

    #[test]
    fn CLITest_Option_TargetType_NegativeIntegerParsing()
    {
        assert_eq!(
            argument_parsing("search -to == 10 --target-type i8 -t -10".to_string()).unwrap().target_type,
            TargetType::i8);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type i16 -t -1".to_string()).unwrap().target_type,
            TargetType::i16);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type i32 -t -5".to_string()).unwrap().target_type,
            TargetType::i32);

        assert_eq!(
            argument_parsing("search -to == 10 --target-type i64 -t -10".to_string()).unwrap().target_type,
            TargetType::i64);

        // -0 is valid
        assert_eq!(
            argument_parsing("search -to == 10 --target-type i8 -t -0".to_string()).unwrap().target_type,
            TargetType::i8);
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

    #[test]
    fn CLITest_Option_TargetOperations()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -t 10".to_string()).unwrap().operations,
            vec![(SearchEngines::ComparisonOperation::Equal, "10".to_string())]);

        assert_eq!(
            argument_parsing("search --target-operation == 15 -t 10".to_string()).unwrap().operations,
            vec![(SearchEngines::ComparisonOperation::Equal, "15".to_string())]);

        assert_eq!(
            argument_parsing("search -to != 10 -t 10".to_string()).unwrap().operations,
            vec![(SearchEngines::ComparisonOperation::Unequal, "10".to_string())]);

        assert_eq!(
            argument_parsing("search -to > 10 -t 10".to_string()).unwrap().operations,
            vec![(SearchEngines::ComparisonOperation::Greater, "10".to_string())]);

        assert_eq!(
            argument_parsing("search -to >= 10 -t 10".to_string()).unwrap().operations,
            vec![(SearchEngines::ComparisonOperation::GreaterOrEqual, "10".to_string())]);

        assert_eq!(
            argument_parsing("search -to < 10 -t 10".to_string()).unwrap().operations,
            vec![(SearchEngines::ComparisonOperation::Less, "10".to_string())]);

        assert_eq!(
            argument_parsing("search -to <= 10 -t 10".to_string()).unwrap().operations,
            vec![(SearchEngines::ComparisonOperation::LessOrEqual, "10".to_string())]);
    }

    #[test]
    fn CLITest_Option_TargetOperations_MunltipleInputs()
    {
        assert_eq!(
            argument_parsing("search -to > 10 -to < 20".to_string()).unwrap().operations,
            vec![
                (SearchEngines::ComparisonOperation::Greater, "10".to_string()),
                (SearchEngines::ComparisonOperation::Less, "20".to_string())
            ]);

        assert_eq!(
            argument_parsing("search -to >= 5 -to <= 25".to_string()).unwrap().operations,
            vec![
                (SearchEngines::ComparisonOperation::GreaterOrEqual, "5".to_string()),
                (SearchEngines::ComparisonOperation::LessOrEqual, "25".to_string())
            ]);
    }

    #[test]
    fn CLITest_Option_TargetOperationsFail()
    {
        assert_eq!(
            argument_parsing("search -to GreaterThan 10 -t 10".to_string()),
            Err(CommandParsingError::InvalidComparisonOperation));
    }

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
    fn CLITest_Option_PagePermissionsAtLeast()
    {
        // Default
        assert_eq!(
            argument_parsing("search -to == 10".to_string()).unwrap().page_permissions_at_least,
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write);

        assert_eq!(
            argument_parsing("search -to == 10 -pal read".to_string()).unwrap().page_permissions_at_least,
            GenericOSInterface::PageProtection_Read);

        assert_eq!(
            argument_parsing("search -to == 10 --page-perms-at-least read|execute".to_string()).unwrap().page_permissions_at_least,
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Execute);

        assert_eq!(
            argument_parsing("search -to == 10 -pal read|write|execute".to_string()).unwrap().page_permissions_at_least,
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write|GenericOSInterface::PageProtection_Execute);
    }

    #[test]
    fn CLITest_Option_PagePermissionsAtLeastFail()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -pal read|not-a-permission".to_string()),
            Err(CommandParsingError::InvalidPagePermission));
    }

    #[test]
    fn CLITest_Option_PagePermissionsExact()
    {
        // Default
        assert_eq!(
            argument_parsing("search -to == 10".to_string()).unwrap().page_permissions_exact,
            None);

        assert_eq!(
            argument_parsing("search -to == 10 -pe read".to_string()).unwrap().page_permissions_exact,
            Some(GenericOSInterface::PageProtection_Read));

        assert_eq!(
            argument_parsing("search -to == 10 --page-perms-exact read|write".to_string()).unwrap().page_permissions_exact,
            Some(GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write));

        assert_eq!(
            argument_parsing("search -to == 10 --page-perms-exact execute|read|write".to_string()).unwrap().page_permissions_exact,
            Some(GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write|GenericOSInterface::PageProtection_Execute));
    }

    #[test]
    fn CLITest_Option_PagePermissionsExactFail()
    {
        assert_eq!(
            argument_parsing("search -to == 10 -pe not-a-permission".to_string()),
            Err(CommandParsingError::InvalidPagePermission));
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
    fn CLITest_Option_FreezeInterval()
    {
        // Default
        assert_eq!(
            argument_parsing("write -aa 1000 -t 10".to_string()).unwrap().freeze_interval_ms,
            1000);

        assert_eq!(
            argument_parsing("write -aa 1000 -t 10 -fzi 999".to_string()).unwrap().freeze_interval_ms,
            999);

        assert_eq!(
            argument_parsing("write -aa 1000 -t 10 --freeze-interval 999".to_string()).unwrap().freeze_interval_ms,
            999);
    }

    #[test]
    fn CLITest_Option_FreezeIntervalFail()
    {
        assert_eq!(
            argument_parsing("write -aa 1000 -t 10 -fzi aaa".to_string()),
            Err(CommandParsingError::InvalidFreezeInterval));
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

    #[test]
    fn CLITest_Option_FilePath()
    {
        assert_eq!(
            argument_parsing("inject -f c:\\windows\\file\\path".to_string()).unwrap().file_path,
            Some("c:\\windows\\file\\path".to_string()));

        assert_eq!(
            argument_parsing("inject --file c:\\windows\\file\\path".to_string()).unwrap().file_path,
            Some("c:\\windows\\file\\path".to_string()));
    }

    #[test]
    fn CLITest_Option_FilePathFail()
    {
        assert_eq!(
            argument_parsing("inject".to_string()),
            Err(CommandParsingError::NoFilePath));
    }

    #[test]
    fn CLITest_Option_DryRun()
    {

        // Default
        assert_eq!(
            argument_parsing("inject -f c:\\windows\\file\\path".to_string()).unwrap().dry_run,
            false);

        assert_eq!(
            argument_parsing("inject -f c:\\windows\\file\\path -dr".to_string()).unwrap().dry_run,
            true);

        assert_eq!(
            argument_parsing("inject -f c:\\windows\\file\\path --dry-run".to_string()).unwrap().dry_run,
            true);
    }

}