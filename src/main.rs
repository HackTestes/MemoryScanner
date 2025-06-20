//Disable some style warnings
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)] 
#![allow(non_snake_case)]
#![allow(unexpected_cfgs)]

// Treat all warnings as errors
//#![deny(warnings)]

use std::env;
use std::io;
use std::io::Write;
use std::time;
use std::time::Duration;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::fs;
mod ThreadPool;
mod WorkloadPartitioning;
mod SearchEngines;
mod GenericOSInterface;
mod Matches;
mod TestOSInterface;
mod WindowsOSInterface;
mod SearchGlue;
mod ResultMergerHelpers;
mod SearchRoutines;
mod CLIFrontEnd;
mod Configuration;
mod CodeInjectionFileParsing;
mod CodeInjection;
mod Tokenization;

fn GetNumberOfMatches(all_matches: &Vec<Matches::AddressMatches>) -> usize
{
    let mut match_num = 0;
    for result_section in all_matches
    {
        match_num += result_section.matches.len();
    }

    return match_num;
}

fn parse_operations<T: std::str::FromStr>(target_operations: Vec<(SearchEngines::ComparisonOperation, String)>) -> Vec<(SearchEngines::ComparisonOperation, T)>
    where T: std::str::FromStr, <T as std::str::FromStr>::Err : std::fmt::Debug // Restrict the type to be able to be a string and be displayed by debug (https://github.com/rust-lang/rust/issues/43262)
{
    let mut parsed_operations = vec![];

    for op_pair in target_operations
    {
        // It shouldn't panic here if the front end did the correct validation
        parsed_operations.push( (op_pair.0, op_pair.1.parse::<T>().unwrap()) );
    }

    return parsed_operations;
}

// This function is here to help me with code reuse (normal action and the freeze option)
fn write_action_subroutine(command_config: &Configuration::Config, process_handle: &mut GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    let write_result = match command_config.target_type
    {
        // Each operation takes the target string (which should have been verified by the parsing),
        // parses it to the right type
        // and then makes it into a byte representation

        // Unsigned integer
        Configuration::TargetType::u8 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<u8>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::u16 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<u16>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::u32 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<u32>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::u64 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<u64>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::u128 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<u128>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        // Signed integer
        Configuration::TargetType::i8 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<i8>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::i16 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<i16>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::i32 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<i32>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::i64 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<i64>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::i128 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<i128>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        // Float
        Configuration::TargetType::f32 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<f32>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),

        Configuration::TargetType::f64 => process_handle.write_into_vm( &command_config.target.clone().unwrap().parse::<f64>().unwrap().to_ne_bytes(), command_config.write_abs_addr.unwrap()),
    };

    // TODO: Report the errors back to the user in a better way (if possible)
    if write_result.is_err()
    {
        eprintln!("Error in writing operation: {:?}", write_result);
        return write_result;
    }

    return Ok(());
}

fn engine_comparator_subroutine(command_config: Configuration::Config, mut results: Vec<Matches::AddressMatches>, process_handle: &GenericOSInterface::GenericProcess) -> Result<Vec<Matches::AddressMatches>, SearchGlue::SearchErrors>
{

    if command_config.engine == SearchEngines::Engines::comparator
    {
        // It we are not filtering, we should start a new search
        if command_config.filter == false
        {
            // Since we are starting a new search, drop the current results immediately
            //results = vec![];
            results.clear();

            return match command_config.target_type
            {
                Configuration::TargetType::u8 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_u8, 
                    parse_operations::<u8>(command_config.operations)
                ),

                Configuration::TargetType::u16 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_u16, 
                    parse_operations::<u16>(command_config.operations)
                ),

                Configuration::TargetType::u32 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_u32, 
                    parse_operations::<u32>(command_config.operations)
                ),

                Configuration::TargetType::u64 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_u64, 
                    parse_operations::<u64>(command_config.operations)
                ),

                Configuration::TargetType::u128 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_u128, 
                    parse_operations::<u128>(command_config.operations)
                ),

                Configuration::TargetType::i8 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_i8, 
                    parse_operations::<i8>(command_config.operations)
                ),

                Configuration::TargetType::i16 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_i16, 
                    parse_operations::<i16>(command_config.operations)
                ),

                Configuration::TargetType::i32 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_i32, 
                    parse_operations::<i32>(command_config.operations)
                ),

                Configuration::TargetType::i64 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_i64, 
                    parse_operations::<i64>(command_config.operations)
                ),

                Configuration::TargetType::i128 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_i128, 
                    parse_operations::<i128>(command_config.operations)
                ),

                Configuration::TargetType::f32 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_f32, 
                    parse_operations::<f32>(command_config.operations)
                ),

                Configuration::TargetType::f64 => SearchGlue::StartSearchComparator(
                    command_config.page_permissions_at_least,
                    command_config.page_permissions_exact,
                    Some(GenericOSInterface::GenericRegionState::Resident), // State - It doesn't make sense to read memory that isn't in RAM
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_Comparator_f64, 
                    parse_operations::<f64>(command_config.operations)
                ),
            };
        }

        // Here we filter the previous matches
        else
        {
            println!("Filtering results");
            return match command_config.target_type
            {
                Configuration::TargetType::u8 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_u8,
                    parse_operations::<u8>(command_config.operations)
                ),

                Configuration::TargetType::u16 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_u16,
                    parse_operations::<u16>(command_config.operations)
                ),

                Configuration::TargetType::u32 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_u32,
                    parse_operations::<u32>(command_config.operations)
                ),

                Configuration::TargetType::u64 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_u64,
                    parse_operations::<u64>(command_config.operations)
                ),

                Configuration::TargetType::u128 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_u128,
                    parse_operations::<u128>(command_config.operations)
                ),

                Configuration::TargetType::i8 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_i8,
                    parse_operations::<i8>(command_config.operations)
                ),

                Configuration::TargetType::i16 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_i16,
                    parse_operations::<i16>(command_config.operations)
                ),

                Configuration::TargetType::i32 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_i32,
                    parse_operations::<i32>(command_config.operations)
                ),

                Configuration::TargetType::i64 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_i64,
                    parse_operations::<i64>(command_config.operations)
                ),

                Configuration::TargetType::i128 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_i128,
                    parse_operations::<i128>(command_config.operations)
                ),

                Configuration::TargetType::f32 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_f32,
                    parse_operations::<f32>(command_config.operations)
                ),

                Configuration::TargetType::f64 => SearchGlue::FilterSearchComparator(
                    results,
                    process_handle,
                    command_config.num_threads,
                    command_config.copy_buffer_size,
                    command_config.thread_storage,
                    SearchEngines::LinearSearch_ComparatorFilter_f64,
                    parse_operations::<f64>(command_config.operations)
                ),
            };
        }
    }

    return Ok(vec![]);
}

fn main_help()
{
    println!("USAGE \n\tMemoryScanner [-h|--help] OR MemoryScanner <TARGET_PID>\n\n");
    CLIFrontEnd::print_CLI_help();
}

fn main()
{
    // Get the arguments from command line
    // We only need to check for help, otherwise I expect a fixed position of args
    let args: Vec<String> = env::args().collect();

    // Does the user need help?
    if &args[1] == "--help" || &args[1] == "-h"
    {
        main_help();
        return;
    }

    // We have verified for help already, use the normal command now
    // PID
    let arg = &args[1];

    // Is the PID valid?
    let process_id_r = arg.parse::<u64>();

    let process_id: u64 = if process_id_r.is_ok(){process_id_r.unwrap()} else
    {
        eprintln!("The process ID given is not valid");
        main_help();
        return;
    };

    // Now attach to the process and verify any errors
    let process_r = GenericOSInterface::GenericProcess::attach(process_id);

    let mut process_handle: GenericOSInterface::GenericProcess = if process_r.is_ok(){process_r.unwrap()} else
    {
        eprintln!("It was not possible to attach to the process. Terminating");
        return;
    };
    println!("Process attached! PID: {}", process_id);

    // Now that the process was SUCCESSFULLY attached...

    // We create our storage for results
    let mut results: Vec<Matches::AddressMatches> = vec![];

    // And the saved results as well
    let mut saved_results: Vec< Vec<Matches::AddressMatches> >= vec![];


    // Main search loop
    loop
    {
        // We can ask to the user for inputs on what to do here
        let command_config: Configuration::Config;
        loop
        {
            // Get the user input
            let mut command = String::new();
            print!("\n -------------------------------------------------- \n\ncommand> ");

            // Clear all previous things in the stdout, so it doesn't get mixed in the command input
            std::io::stdout().flush().unwrap();

            // Read the command
            io::stdin().read_line(&mut command).expect("failed to read line");

            print!("\n");

            // Parse commands and return a config
            let command_result = CLIFrontEnd::argument_parsing( String::from(command.trim()) );

            // If we get a valid command
            if command_result.is_ok()
            {
                command_config = command_result.unwrap();
                break;
            }
        }

        // Do what the user asked based on the configuration

        // Early help
        if command_config.help == true
        {
            // There is no need to print anything here, the argument parsing will do it already
            continue;
        }

        match command_config.action
        {
            // This works in the same way as the help option
            CLIFrontEnd::ActionsEnum::Help => continue,

            CLIFrontEnd::ActionsEnum::Search =>
            {
                println!("Starting search");
                let timer = time::Instant::now();

                let search_results_r:Result<Vec<Matches::AddressMatches>, SearchGlue::SearchErrors> = match command_config.engine
                {
                    SearchEngines::Engines::comparator =>
                    {
                        // Pause the process before interacting with it
                        let _pause_process_tracker = process_handle.tracked_pause();

                        // I clone the value here because in case of errors it might not be initialized
                        // So this is a poor's man save, so the user doesn't lose its search
                        engine_comparator_subroutine(command_config, results.clone(), &process_handle)
                    },

                    SearchEngines::Engines::exact =>
                    {
                        eprintln!("Exact engine not supported yet!");
                        continue;
                    },
                };

                if search_results_r.is_ok()
                {
                    // We found something, then save it
                    let search_time = timer.elapsed();
                    results = search_results_r.unwrap();

                    // User output
                    print!("\n"); // Minor formatting
                    println!("Total search time:\n {}s\n {}ms\n {}us\n", search_time.as_secs(), search_time.as_millis(), search_time.as_micros());
                    println!("{} matches found\n", GetNumberOfMatches(&results));
                }
                else
                {
                    // An error occurred, tell the user
                    println!("Error in the search: {:?}", search_results_r);
                };
            },

            CLIFrontEnd::ActionsEnum::Display =>
            {
                println!("Number of sections: {}\n", &results.len());

                for result_section in &results
                {
                    match command_config.display_style
                    {
                        Matches::MatchDisplayStyle::Hex =>
                        {
                            println!("Matches addresses: {} \n", &result_section.display_matches(command_config.display_style.clone()));
                        },

                        Matches::MatchDisplayStyle::Decimal =>
                        {
                            println!("Matches addresses: {} \n", &result_section.display_matches(command_config.display_style.clone()));
                        }
                    }
                }
            },

            CLIFrontEnd::ActionsEnum::Write =>
            {
                if command_config.freeze == false
                {
                    let write_r = write_action_subroutine(&command_config, &mut process_handle);

                    if write_r.is_ok()
                    {
                        println!("Write successful!");
                    }
                }

                else
                {
                    // Create an atomic to signal thread stop
                    // An atomic is easier to share, but we don't care about race conditions in this case
                    let should_thread_stop = Arc::new(AtomicBool::new(false));

                    // The thread can only read the atomic
                    let should_thread_stop_read = Arc::clone(&should_thread_stop);

                    // We must also share the handle with the thread
                    // This will take ownership of the handle, so we must give it back
                    let process_handle_arc_main = Arc::new(Mutex::new(process_handle));
                    let process_handle_arc_thread = process_handle_arc_main.clone();

                    let config_clone = command_config.clone();

                    let thread_join_handle = thread::spawn(move ||
                    {
                        let millis = Duration::from_millis(config_clone.freeze_interval_ms);
                        loop
                        {
                            // write memory
                            // There is no error checking here, this means that any write error is ignored
                            let _ = write_action_subroutine(&config_clone, &mut process_handle_arc_thread.lock().unwrap());
                
                            // sleep
                            thread::sleep(millis);
                
                            // read atomic
                            if (should_thread_stop_read.load(Ordering::Relaxed)) == true
                            {
                                break;
                            }
                        }
                    });
                
                    let mut command = String::new();
                    println!("Press ENTER to stop freeze and continue");
                    std::io::stdout().flush().unwrap();
                    io::stdin().read_line(&mut command).expect("failed to read line");
                
                    // only the main thread can write
                    should_thread_stop.store(true, Ordering::Relaxed);

                    // Ignore errors here - but if something goes bad, crash loudly
                    let _res = thread_join_handle.join().unwrap();

                    // Now take the handle back
                    process_handle = Arc::try_unwrap(process_handle_arc_main).unwrap().into_inner().unwrap();
                }
            },

            CLIFrontEnd::ActionsEnum::Save =>
            {
                // Copy the results to the saved buffer, leaving the original results untouched
                saved_results.push(results.clone());
                println!("Results saved! Now with: {} results", saved_results.len());
            },

            CLIFrontEnd::ActionsEnum::Restore =>
            {
                if command_config.restore_entry == None
                {
                    // Similar to pop, but doesn't remove
                    // The user might need to reuse such result, so don't remove it
                    results = saved_results[saved_results.len()-1].clone();
                    println!("Restored the last saved result");
                }

                else
                {
                    let entry_idx: usize = command_config.restore_entry.unwrap();

                    if entry_idx < saved_results.len()
                    {
                        results = saved_results[entry_idx].clone();
                        println!("Restored entry {}", entry_idx);
                    }
                    else
                    {
                        eprintln!("Entry index is too big");
                    }
                }
            },

            CLIFrontEnd::ActionsEnum::Remove =>
            {
                if command_config.remove_all_saved_entries == false
                {
                    saved_results.pop();
                    println!("Last saved result removed. Now with: {}", saved_results.len());
                }

                else
                {
                    saved_results.clear();
                    println!("All saved results removed. Now with: {}", saved_results.len());
                }
            },

            CLIFrontEnd::ActionsEnum::Inject => 
            {
                // Read the injection file
                let file_contents_r = fs::read_to_string(command_config.file_path.unwrap() );

                if file_contents_r.is_err()
                {
                    eprintln!("Could not open the file or read it. Error: {:?}", file_contents_r.unwrap());
                    continue;
                }

                // Parse it 
                let injection_config_r = CodeInjectionFileParsing::parse_injection_file(file_contents_r.unwrap());

                if injection_config_r.is_err()
                {
                    eprintln!("Could not parse the code injection file. Error: {:?}", injection_config_r);
                    continue;
                }

                let injection_config = injection_config_r.unwrap();

                // Inject code
                let code_injection_r = CodeInjection::main_code_injection_flow(injection_config, &mut process_handle, command_config.dry_run, true);

                if code_injection_r.is_err()
                {
                    eprintln!("Error during code injection. Error: {:?}", code_injection_r);
                    continue;
                }
            },

            CLIFrontEnd::ActionsEnum::Exit => break,
        }
    }
}
