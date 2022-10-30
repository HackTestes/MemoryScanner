use std::collections::HashMap;
use std::any::Any;

#[derive(Clone)]
pub struct Arg
{
    long_option: String,
    short_option: String,
    parameters: Vec<String>,
    description: String,
    action: fn(opt: &[Self], parameters: Vec<String>, index: usize),
}

impl Arg
{
    pub fn new(long_opt: String, short_opt: String, arg_para: Vec<String>, desc: String, function: fn(opt: &[Self], parameters: Vec<String>, index: usize)) -> Self
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

pub fn ParseArg(command: String)
{
    // Holds all program arguments
    let program_args = [
        Arg::new("-h".to_string(), "--help".to_string(), [].to_vec(), "Displays help text".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize|
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
                    parameters = format!("<{}>", opt_arg.parameters.join("><").to_uppercase());
                }

                final_help_text.push_str(&format!("{} {} {}\n\t{}\n\n", opt_arg.short_option.clone(), opt_arg.long_option.clone(), parameters, opt_arg.description.clone()));
            }

            println!("{}", final_help_text);
        }),

        Arg::new("-f".to_string(), "--filter".to_string(), ["value_type".to_string()].to_vec(), "Selects which data types should be used in the filters".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize|
        {
            println!("FILTER TEXT");
        }),

        Arg::new("-j".to_string(), "--jobs".to_string(), ["number_of_threads".to_string()].to_vec(), "Selects the maximum number of worker threads".to_string(), |opt: &[Arg], parameters: Vec<String>, index: usize|
        {
            println!("JOBS TEXT");
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
        if option.len() > 1
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
            }

            // Option exists
            else
            {
                let argument = &program_args[arg_index.unwrap().clone()];
                let necessary_number_parameters = argument.parameters.len();

                if parameters.len() < necessary_number_parameters
                {
                    eprintln!("Wrong number of parameters");
                }

                // Everything is OK: it exists and has the right parameters
                else
                {
                    (argument.action)(&program_args, parameters, 0 as usize);
                }
            }
        }

        // Not an option
        else
        {
            continue;
        }
    }

}
