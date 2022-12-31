use std::env;
use std::io;
mod args_parse;
mod Config;
mod ReadMemory;

use windows_sys::{
    Win32::System::Threading::*, Win32::Foundation::*,
    Win32::System::Memory::*, core::*,
};

fn main()
{
    let args: Vec<String> = env::args().collect();
    let arg = &args[1];

    let process_id: u32 = arg.parse::<u32>().unwrap();

    let mut guess = String::new();
 
    args_parse::ParseArg("-N --filter=u32,u64 -J=12 --help".to_string());
    //io::stdin().read_line(&mut guess).expect("failed to readline");

    unsafe
    {
        let process_handle: windows_sys::Win32::Foundation::HANDLE = windows_sys::Win32::System::Threading::OpenProcess(windows_sys::Win32::System::Threading::PROCESS_QUERY_INFORMATION |
                                                                        windows_sys::Win32::System::Threading::PROCESS_VM_OPERATION |
                                                                        windows_sys::Win32::System::Threading::PROCESS_VM_READ |
                                                                        windows_sys::Win32::System::Threading::PROCESS_VM_WRITE,
                                                                        0, // False
                                                                        process_id);

        println!("Process attached! Id- {}", process_handle);

        // main loop
        loop
        {
            // Get the command
            let mut command = String::new();
            io::stdin().read_line(&mut command).expect("failed to readline");

            // Parse commands and return a config
            let command_config = args_parse::ParseArg(command).unwrap();

            // Store the results from searches
            let mut result = Vec::new();

            // Execute the right action
            match command_config.action
            {
                // Memory search
                Config::Action::Search =>
                {
                    if command_config.scan_start == Config::ScanStartFlag::NewScan
                    {
                        result = ReadMemory::SearchProcessMemory_Initial(command_config.filters, command_config.thread_count, command_config.value_to_search, process_handle);
                    }
                    else
                    {
                        result = ReadMemory::FilterMatches(result, command_config.filters, command_config.thread_count, command_config.value_to_search, process_handle);
                    }
                },

                // Memory writes
                Config::Action::WriteMemory =>
                {
                    println!("WRITE PLACEHOLDER!");
                },

                // Display the results to the user
                Config::Action::Display =>
                {
                    println!("Number of sections: {}\n", &result.len());

                    for result_section in result
                    {
                        println!("Section: {} \n\n\tMatches addresses: {:?} \n", &result_section.matches.len(), &result_section.get_absolute_virtual_address());
                    }
                },

                // Exit the program
                Config::Action::Exit => break,
            }
        }

        windows_sys::Win32::Foundation::CloseHandle(process_handle);
    }
}
