use std::env;
use std::io;
mod args_parse;
mod Config;
mod ReadMemory;

use windows_sys::{
    Win32::System::Threading::*, Win32::Foundation::*,
    Win32::System::Memory::*, core::*,
};

fn ReadPageInfo(mut page_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION)
{
    print!("Base Address: {} -- RegionSize: {}\t", page_info.BaseAddress as u64, page_info.RegionSize);

    match page_info.State
    {
        windows_sys::Win32::System::Memory::MEM_COMMIT => print!("Committed"),
        windows_sys::Win32::System::Memory::MEM_RESERVE => print!("Reserved"),
        windows_sys::Win32::System::Memory::MEM_FREE => print!("Free"),
        _=> print!("Empty State"),
    }

    print!("\t");

    match page_info.Type
    {
        windows_sys::Win32::System::Memory::MEM_IMAGE => print!("Code Module"),
        windows_sys::Win32::System::Memory::MEM_MAPPED => print!("Mapped"),
        windows_sys::Win32::System::Memory::MEM_PRIVATE => print!("Private"),
        _=> print!("Empty Type"),
    }

    print!("\t");

    if (page_info.AllocationProtect & windows_sys::Win32::System::Memory::PAGE_NOCACHE) != 0
    {
        print!("non-cacheable");
    } 
    if (page_info.AllocationProtect & windows_sys::Win32::System::Memory::PAGE_GUARD) != 0
    {
        print!("guard page");
    }

    print!("\t");

    page_info.AllocationProtect &= !(windows_sys::Win32::System::Memory::PAGE_GUARD | windows_sys::Win32::System::Memory::PAGE_NOCACHE);

    match page_info.AllocationProtect
    {
        windows_sys::Win32::System::Memory::PAGE_READONLY => print!("Read Only"),
        windows_sys::Win32::System::Memory::PAGE_READWRITE => print!("Read/Write"),
        windows_sys::Win32::System::Memory::PAGE_WRITECOPY => print!("Copy on Write"),
        windows_sys::Win32::System::Memory::PAGE_EXECUTE => print!("Execute only"),
        windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ => print!("Execute/Read"),
        windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE => print!("Execute/Read/Write"),
        windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY => print!("COW Executable"),
        _=> print!("Empty AllocationProtect"),
    }

    print!("\n\n");
}


fn GetMemoryPageInfo(mut process: windows_sys::Win32::Foundation::HANDLE) -> u64
{
    unsafe
    {
        let mut usage: u64 = 0;

        let mut p: *mut std::os::raw::c_void = std::ptr::null_mut();

        let mut info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: 0,
            Type: 0,
        };

        let mut bytes = windows_sys::Win32::System::Memory::VirtualQueryEx(process, p, &mut info, std::mem::size_of::<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION>());
        println!("Entering Loop");
        
        while windows_sys::Win32::System::Memory::VirtualQueryEx(process, p, &mut info, std::mem::size_of::<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION>()) == std::mem::size_of::<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION>()
        {
            ReadPageInfo(info);

            if info.AllocationProtect == windows_sys::Win32::System::Memory::PAGE_READWRITE
            {
                //println!("Read write (execute) page");
            }

            p = info.BaseAddress;
            p = p.add(info.RegionSize);
        }
        return usage;
    }
}

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

        println!("Process: {}", process_handle);

        GetMemoryPageInfo(process_handle);

        windows_sys::Win32::Foundation::CloseHandle(process_handle);
    }

    let mut buffer: [u8; 5] = [0; 5];

    // 15 in little endian
    buffer[0] = 15;
    buffer[1] = 0;
    buffer[2] = 0;
    buffer[3] = 0;
    buffer[4] = 0;

    let ptr : *const u8 = buffer.as_ptr();

    unsafe
    {
        let foo = *ptr as u32;
        println!("Var: {}", u32::from_ne_bytes(buffer[0..4].try_into().unwrap()));

        // foo must change, if not it must be a copy
        //buffer[0] = 0;
        println!("Var: {}", *ptr as u32);
    }
    
    for x in buffer
    {
        print!("{x} ");
    }

}
