use std::env;
use std::io;
use std::io::Write;
mod args_parse;
mod Config;
mod ReadMemory;
mod WriteMemory;
mod ThreadPool;
mod ThreadPool_v2;
mod ThreadPool_v3;

use windows_sys::{
    Win32::System::Threading::*, Win32::Foundation::*,
    Win32::System::Memory::*, core::*,
};

fn GetNumberOfMatches(all_matches: &Vec<ReadMemory::MemoryMatches>) -> usize
{
    let mut match_num = 0;
    for result_section in all_matches
    {
        match_num += result_section.matches.len();
    }

    return match_num;
}

fn main()
{
    let args: Vec<String> = env::args().collect();
    let arg = &args[1];

    let process_id: u32 = arg.parse::<u32>().unwrap();

    unsafe
    {
        let process_handle: windows_sys::Win32::Foundation::HANDLE = windows_sys::Win32::System::Threading::OpenProcess(windows_sys::Win32::System::Threading::PROCESS_QUERY_INFORMATION |
                                                                        windows_sys::Win32::System::Threading::PROCESS_VM_OPERATION |
                                                                        windows_sys::Win32::System::Threading::PROCESS_VM_READ |
                                                                        windows_sys::Win32::System::Threading::PROCESS_VM_WRITE,
                                                                        0, // False
                                                                        process_id);

        println!("Process attached! Id- {}", process_id);

        // Store the results from searches
        let mut result: Vec<ReadMemory::MemoryMatches> = Vec::new();

        // main loop
        loop
        {
            let command_config;
            loop
            {
                // Get the command
                let mut command = String::new();
                print!("\n -------------------------------------------------- \n\ncommand> ");
                std::io::stdout().flush().unwrap();
                io::stdin().read_line(&mut command).expect("failed to readline");

                print!("\n");

                // Parse commands and return a config
                let command_result = args_parse::ParseArg( String::from(command.trim()) );

                if command_result.is_ok()
                {
                    command_config = command_result.unwrap();
                    break;
                }
            }

            if command_config.help == true
            {
                continue;
            }

            // Execute the right action
            match command_config.action
            {
                // Memory search
                Config::Action::Search =>
                {
                    println!("Search Started!");
                    if command_config.scan_start == Config::ScanStartFlag::NewScan
                    {
                        result = ReadMemory::SearchProcessMemory_Initial(command_config.filters, command_config.thread_count, command_config.value_to_search, process_handle);
                        println!("Number of matches: {}", GetNumberOfMatches(&result));
                    }
                    else
                    {
                        result = ReadMemory::FilterMatches(&mut result, command_config.filters, command_config.thread_count, command_config.value_to_search, process_handle);
                        println!("Number of matches: {}", GetNumberOfMatches(&result));
                    }
                },

                // Memory writes
                Config::Action::WriteMemory =>
                {
                    let success = WriteMemory::WriteIntoProcessMemory_EntryPoint(process_handle, &result, command_config.value_to_search, command_config.freeze, command_config.sleep_time, command_config.filters[0].clone());

                    if success.is_ok() {println!("Write successful!");} else {println!("Error on write!");}
                },

                // Display the results to the user
                Config::Action::Display =>
                {
                    println!("Number of sections: {}\n", &result.len());

                    for result_section in &result
                    {
                        println!("Section matches: {} \n\n\tMatches addresses: {:?} \n", &result_section.matches.len(), &result_section.get_absolute_virtual_address());
                    }
                },

                // Exit the program
                Config::Action::Exit => break,
            }

        }

        windows_sys::Win32::Foundation::CloseHandle(process_handle);
    }
}
