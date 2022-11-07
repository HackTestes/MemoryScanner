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
    action: fn(opt: &[Self], parameters: Vec<String>, index: usize) -> Result<(), ArgError>,
}

impl Arg
{
    pub fn new(long_opt: String, short_opt: String, arg_para: Vec<String>, desc: String, function: fn(opt: &[Self], parameters: Vec<String>, index: usize) -> Result<(), ArgError>) -> Self
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

pub fn ParseArg(command: String) -> Result<(), ArgError>
{
    // Holds all program arguments
    let program_args = [
        Arg::new("-h".to_string(), "--help".to_string(), [].to_vec(), "Displays help text".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize| -> Result<(), ArgError> 
        {
            let mut final_help_text: String = String::new();

            // Main section
            final_help_text.push_str("[MAIN]\n\tMemoryScanner <PROCESS_ID> [OPTIONS]\n\n");

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
            return Ok(());
        }),

        Arg::new("-f".to_string(), "--filter".to_string(), ["value_types...".to_string()].to_vec(), "Selects which data types should be used in the filters".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize| -> Result<(), ArgError>
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

            println!("{:?}", filter_list);

            return Ok(());
        }),

        Arg::new("-j".to_string(), "--jobs".to_string(), ["number_of_threads".to_string()].to_vec(), "Selects the maximum number of worker threads".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize| -> Result<(), ArgError>
        {
            println!("JOBS TEXT");
            return Ok(());
        }),

        Arg::new("-s".to_string(), "--scanType".to_string(), ["scan_type".to_string()].to_vec(), "Chooses how the search is performed (exact or unknown value)".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize| -> Result<(), ArgError>
        {
            println!("JOBS TEXT");
            return Ok(());
        }),

        Arg::new("-s".to_string(), "--scanStart".to_string(), ["start".to_string()].to_vec(), "Chooses whether the program should make a new scan or not".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize| -> Result<(), ArgError>
        {
            println!("JOBS TEXT");
            return Ok(());
        }),

        Arg::new("-v".to_string(), "--value".to_string(), ["value".to_string()].to_vec(), "Value that should be searched".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize| -> Result<(), ArgError>
        {
            println!("JOBS TEXT");
            return Ok(());
        })
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

    println!("HashMap: {:?}", args_map);

    let command_list: Vec<_> = command.split(" ").collect();

    // Match the corresponding option in the HashMap
    // Peforms the action and make checks
    for (index, word) in command_list.iter().enumerate()
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

                println!("Parameters:{:?}", parameters);

                if parameters.len() < necessary_number_parameters
                {
                    eprintln!("Wrong number of parameters");
                    return Err(ArgError::InsufficientParameters);
                }

                // Everything is OK: it exists and has the right parameters
                else
                {
                    match (argument.action)(&program_args, parameters, 0 as usize)
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

    return Ok(());
}



// Unit test

#[cfg(test)]
mod tests
{
    use crate::args_parse::*;

    #[test]
    fn IncorrectCommand()
    {
        assert_eq!( Err(ArgError::OptionDoesntExist), ParseArg("-N --filter=u32,u64 -J=12 --help".to_string()) );
    }

    #[test]
    fn InsufficientParametersCommand()
    {
        assert_eq!( Err(ArgError::InsufficientParameters), ParseArg("--filter= ".to_string()) );
    }

    #[test]
    fn ReadMemoryCommand()
    {
        assert_eq!( Ok(()), ParseArg("--filter=u32,u64 -j=12 --help".to_string()) );
    }

}