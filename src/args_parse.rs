use std::collections::HashMap;
use std::any::Any;
use crate::Config::*;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum ArgError
{
    OptionDoesntExist,
    InsufficientParameters,
    InvalidCommand
}

#[derive(Clone)]
pub struct Arg
{
    long_option: String,
    short_option: String,
    parameters: Vec<String>,
    description: String,
    action: fn(opt: &[Self], parameters: Vec<String>, config: &mut Configuration) -> Result<(), ArgError>,
}

impl Arg
{
    pub fn new(long_opt: String, short_opt: String, arg_para: Vec<String>, desc: String, function: fn(opt: &[Self], parameters: Vec<String>, config: &mut Configuration) -> Result<(), ArgError>) -> Self
    {
        return Arg
        {
            long_option: long_opt,
            short_option: short_opt,
            parameters: arg_para,
            description: desc,
            action: function
        }
    }
}

pub fn ParseArg(command: String) -> Result<Configuration, ArgError>
{
    // Holds all program arguments
    let program_args = [

        // HELP
        Arg::new("-h".to_string(), "--help".to_string(), [].to_vec(), "Displays help text".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError> 
        {
            let mut final_help_text: String = String::new();

            // Main section
            final_help_text.push_str("[MAIN]\n\tMemoryScanner <PROCESS_ID>\n\n");

            final_help_text.push_str("[COMMAND]\n\t<COMMAND> [OPTIONS]\n");

            final_help_text.push_str("\tCOMMANDS = search, write, display, exit\n\n");

            // Options section
            final_help_text.push_str("[OPTIONS]\n");
            for opt_arg in opt
            {
                let mut parameters: String = "".to_string();
                if opt_arg.parameters.len() > 0
                {
                    parameters = format!("=<{}>", opt_arg.parameters.join("><").to_uppercase());
                }

                final_help_text.push_str(&format!("{} {}{}\n\t{}\n\n", opt_arg.short_option.clone(), opt_arg.long_option.clone(), parameters, opt_arg.description.clone()));
            }

            println!("{}", final_help_text);
            config.help = true;
            return Ok(());
        }),

        // FILTER
        Arg::new("-D".to_string(), "--dataType".to_string(), ["value_types...".to_string()].to_vec(), "Selects which data types should be used in the filters".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError>
        {
            let mut filter_list: Vec<FilterOption> = [].to_vec();

            for parameter in parameters
            {
                match &parameter[..]
                {
                    "u32" => filter_list.push(FilterOption::U32),
                    "u64" => filter_list.push(FilterOption::U64),
                    "i32" => filter_list.push(FilterOption::I32),
                    "i64" => filter_list.push(FilterOption::I64),
                    "f32" => filter_list.push(FilterOption::F32),
                    "f64" => filter_list.push(FilterOption::F64),
                    _=> eprintln!("Wrong filter type")
                }
            }

            //println!("{:?}", filter_list);
            config.filters = filter_list;

            return Ok(());
        }),

        // NUMBER OF THREADS
        Arg::new("-j".to_string(), "--jobs".to_string(), ["number_of_threads".to_string()].to_vec(), "Selects the maximum number of worker threads".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError>
        {
            config.thread_count = parameters[0].parse::<usize>().unwrap();
            return Ok(());
        }),

        /*// TYPE OF SCAN
        Arg::new("-s".to_string(), "--scanType".to_string(), ["scan_type".to_string()].to_vec(), "Chooses how the search is performed (exact or unknown value)".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError>
        {
            println!("JOBS TEXT");
            return Ok(());
        }),*/

        // NEW SCAN OR NOT
        Arg::new("-F".to_string(), "--filterSearch".to_string(), [].to_vec(), "Filter the previous results (Default is to perform a new search)".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError>
        {
            config.scan_start = ScanStartFlag::ReuseResults;
            return Ok(());
        }),

        // VALUE TO SEARCH
        Arg::new("-v".to_string(), "--value".to_string(), ["value".to_string()].to_vec(), "Value that should be searched".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError>
        {
            config.value_to_search = parameters[0].clone();
            return Ok(());
        }),

        // SLEEP VALUE
        Arg::new("-s".to_string(), "--sleep".to_string(), ["miliseconds".to_string()].to_vec(), "Sleep time for write threads (requires freeze)".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError>
        {
            config.sleep_time = parameters[0].parse::<u64>().unwrap();
            return Ok(());
        }),

        // FREEZE TIME
        Arg::new("-S".to_string(), "--freeze".to_string(), [].to_vec(), "Freeze a value in memory using a write loop (use sleep time to configure intervals)".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError>
        {
            config.freeze = true;
            return Ok(());
        }),

        // SELECT ADDRESS - DEFAULT: WRITE TO ALL ADDRESSES
        Arg::new("-a".to_string(), "--address".to_string(), ["address_hex".to_string()].to_vec(), "Select an memory address to write (Default is to write to all matches)".to_string(), |opt: &[Arg], parameters: Vec<String>, config: &mut Configuration| -> Result<(), ArgError>
        {
            config.address_selected = parameters[0].parse::<usize>().unwrap();
            return Ok(());
        }),
    ];

    // Maps the array into a HashMap
    let mut args_map = HashMap::<String, usize>::new();

    for (index, arg) in program_args.iter().enumerate()
    {
        args_map.insert (
            arg.long_option.clone(),
            index
        );

        args_map.insert (
            arg.short_option.clone(),
            index
        );
    }

    //println!("HashMap: {:?}", args_map);

    let mut config: Configuration =  Configuration::new();

    let command_list: Vec<_> = command.split(" ").collect();

    config.action = match command_list[0]
    {
        "search" => Action::Search,
        "write" => Action::WriteMemory,
        "display" => Action::Display,
        "exit" => Action::Exit,
        _=>
        {
            println!("Invalid command: '{}'", command_list[0]);
            return Err(ArgError::InvalidCommand);
        }, 
    };

    // Match the corresponding option in the HashMap
    // Peforms the action and make checks
    for (index, word) in command_list[1..].iter().enumerate()
    {
        let option: Vec<_> = word.split("=").collect();
        let parameters: Vec<String>;

        // Collect the parameters if they exist
        if option.len() > 1 && option[1] != ""
        {
            parameters = option[1].split(",").map(str::to_string).collect();
        }
        else
        {
            parameters = [].to_vec();
        }

        // If it IS an option
        if option[0].starts_with("-") | option[0].starts_with("--")
        {
            let arg_index = args_map.get(option[0]);

            if arg_index == None
            {
                eprintln!("This option doesn't exist: '{}'", option[0]);
                return Err(ArgError::OptionDoesntExist);
            }

            // Option exists
            else
            {
                let argument = &program_args[arg_index.unwrap().clone()];
                let necessary_number_parameters = argument.parameters.len();

                //println!("Parameters:{:?}", parameters);

                if parameters.len() < necessary_number_parameters
                {
                    eprintln!("Wrong number of parameters");
                    return Err(ArgError::InsufficientParameters);
                }

                // Everything is OK: it exists and has the right parameters
                else
                {
                    match (argument.action)(&program_args, parameters, &mut config)
                    {
                        Ok(()) => {},
                        Err(err) => eprintln!("Arg Error: {:?}", err),
                    }
                }
            }
        }

        // Not an option
        else
        {
            continue;
        }
    }

    return Ok(config);
}



// Unit test
#[cfg(test)]
mod tests
{
    use crate::args_parse::*;

    // INCORRECT COMMAND
    #[test]
    fn IncorrectOpt()
    {
        assert_eq!( Err(ArgError::OptionDoesntExist), ParseArg("search --Incorrect --dataType=u32,u64 -J=12 --help".to_string()) );
    }

    #[test]
    fn InsufficientParametersCommand()
    {
        assert_eq!( Err(ArgError::InsufficientParameters), ParseArg("search --dataType= ".to_string()) );
    }

    #[test]
    fn IncorrectCommand()
    {
        assert_eq!( Err(ArgError::InvalidCommand), ParseArg("incorrect_command".to_string()) );
    }

    // HELP ONLY

    // READ MEM
    #[test]
    fn ReadMemorySearchOnly()
    {
        assert_eq!( Ok(Configuration::new()), ParseArg("search".to_string()) );
    }

    #[test]
    fn ReadMemoryCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.filters = vec![FilterOption::U32, FilterOption::U64];
        config.thread_count = 12;

        assert_eq!( Ok(config), ParseArg("search --dataType=u32,u64 -j=12".to_string()) );
    }

    #[test]
    fn ReadMemoryFilterSearchCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.scan_start = ScanStartFlag::ReuseResults;
        config.filters = vec![FilterOption::U32, FilterOption::U64];
        config.thread_count = 12;

        assert_eq!( Ok(config), ParseArg("search --filterSearch --dataType=u32,u64 -j=12".to_string()) );
    }

    #[test]
    fn ReadMemoryFilterSearhValueCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.value_to_search = "10".to_string();
        config.scan_start = ScanStartFlag::ReuseResults;
        config.filters = vec![FilterOption::U32, FilterOption::U64];
        config.thread_count = 12;

        assert_eq!( Ok(config), ParseArg("search --filterSearch --dataType=u32,u64 -j=12 --value=10".to_string()) );
    }
    
    // WRITE MEM
    #[test]
    fn WriteMemoryCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.action = Action::WriteMemory;
        config.value_to_search = "10".to_string();

        assert_eq!( Ok(config), ParseArg("write --value=10".to_string()) );
    }

    #[test]
    fn WriteMemoryFreezeCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.action = Action::WriteMemory;
        config.value_to_search = "10".to_string();
        config.freeze = true;

        assert_eq!( Ok(config), ParseArg("write --freeze --value=10".to_string()) );
    }

    #[test]
    fn WriteMemoryFreezeSleepCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.action = Action::WriteMemory;
        config.value_to_search = "10".to_string();
        config.freeze = true;
        config.sleep_time = 100;

        assert_eq!( Ok(config), ParseArg("write --freeze --sleep=100 --value=10".to_string()) );
    }

    #[test]
    fn WriteMemoryAddressCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.action = Action::WriteMemory;
        config.value_to_search = "10".to_string();
        config.freeze = true;
        config.sleep_time = 100;
        config.address_selected = 1000;

        assert_eq!( Ok(config), ParseArg("write --freeze --sleep=100 --address=1000 --value=10".to_string()) );
    }

    // DISPLAY
    #[test]
    fn DispalyCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.action = Action::Display;

        assert_eq!( Ok(config), ParseArg("display".to_string()) );
    }

    // EXIT
    #[test]
    fn ExitCommand()
    {
        let mut config: Configuration = Configuration::new();
        config.action = Action::Exit;

        assert_eq!( Ok(config), ParseArg("exit".to_string()) );
    }
}