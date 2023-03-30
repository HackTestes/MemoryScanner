use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows_sys::Win32::Foundation::HANDLE;
use std::os::raw::c_void;
use std::mem::size_of;
use std::{thread, time};
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;
use std::io::Write;
use crate::Config::*;
use crate::ReadMemory::*;

macro_rules! WriteProcessMemory
{
    ($data_type:ty, $target_value:expr, $addresses:expr, $process_handle:expr) =>
    {
        |target_value: String, absolute_addresses: Vec<usize>, process_handle: HANDLE| -> i32
        {
            let mut result_code: i32 = 1;

            for base_address in absolute_addresses
            {
                let value_to_be_copied: $data_type = target_value.parse::<$data_type>().unwrap();
                let value_ptr: *const c_void = std::array::from_ref(&value_to_be_copied).as_ptr() as *const c_void;

                let base_address_ptr: *const c_void = base_address as *const c_void;

                let mut bytes_written: usize = 0;
                let bytes_written_ptr: *mut usize = &mut bytes_written;

                if unsafe{ WriteProcessMemory(process_handle, base_address_ptr, value_ptr, size_of::<$data_type>(), bytes_written_ptr)} == 0
                {
                    result_code = 0;
                }
            }
            return result_code;

        }($target_value, $addresses, $process_handle)
    }
}

fn WriteIntoProcessMemory_Start(process_handle: HANDLE, result: &Vec<MemoryMatches>, value: String, data_type: FilterOption) -> Result<(), i32>
{
    for mem_section in result
    {
        let result_code: i32 = match data_type
        {
            FilterOption::U8 => WriteProcessMemory!( u8, value.clone(), mem_section.get_absolute_virtual_address().U8, process_handle ),
            FilterOption::U16 => WriteProcessMemory!( u16, value.clone(), mem_section.get_absolute_virtual_address().U16, process_handle ),
            FilterOption::U32 => WriteProcessMemory!( u32, value.clone(), mem_section.get_absolute_virtual_address().U32, process_handle ),
            FilterOption::U64 => WriteProcessMemory!( u64, value.clone(), mem_section.get_absolute_virtual_address().U64, process_handle ),
            FilterOption::I32 => WriteProcessMemory!( i32, value.clone(), mem_section.get_absolute_virtual_address().I32, process_handle ),
            FilterOption::I64 => WriteProcessMemory!( i64, value.clone(), mem_section.get_absolute_virtual_address().I64, process_handle ),
            FilterOption::F32 => WriteProcessMemory!( f32, value.clone(), mem_section.get_absolute_virtual_address().F32, process_handle ),
            FilterOption::F64 => WriteProcessMemory!( f64, value.clone(), mem_section.get_absolute_virtual_address().F64, process_handle ),
        };

        if result_code != 0
        {
            return Ok(());
        }
    }
    
    return Err(0);
}

fn FreezeMemory(process_handle: HANDLE, result: &Vec<MemoryMatches>, value: String, data_type: FilterOption, sleep_time: u64) -> Result<(), i32>
{
    let stop_thread = Arc::new(AtomicBool::new(false));

    // the thread can read only
    let stop_thread_read =  Arc::clone(&stop_thread);

    let mut result_clone = Vec::with_capacity( result.capacity() );
    result_clone.extend_from_slice(result);
    
    let thread_join_handle = thread::spawn(move ||
    {
        let millis = time::Duration::from_millis(sleep_time);
        let mut success_code = false;

        loop
        {
            // write memory
            success_code = WriteIntoProcessMemory_Start(process_handle.clone(), &result_clone, value.clone(), data_type.clone()).is_ok();

            // sleep
            thread::sleep(millis);

            // read mutex
            if (stop_thread_read.load(Ordering::Relaxed)) == true
            {
                break;
            }
        }

        return success_code;
    });

    let mut command = String::new();
    println!("Press ENTER to stop freeze and continue");
    std::io::stdout().flush().unwrap();
    io::stdin().read_line(&mut command).expect("failed to readline");

    // only the main thread can write
    stop_thread.store(true, Ordering::Relaxed);

    let res = thread_join_handle.join().is_ok();

    if res == true
    {
        return Ok(());
    }
    return Err(0);
}

// Entry point
pub fn WriteIntoProcessMemory_EntryPoint(process_handle: HANDLE, result: &Vec<MemoryMatches>, value: String, freeze: bool, sleep_time: u64, data_type: FilterOption) -> Result<(), i32>
{
    let mut success: bool = false;

    if freeze == true
    {
        success = FreezeMemory(process_handle, result, value, data_type, sleep_time).is_ok();
    }
    else
    {
        success = WriteIntoProcessMemory_Start(process_handle, result, value, data_type).is_ok();
    }

    if success == true
    {
        return Ok(());
    }
    return Err(0);
}

// Unit test
#[cfg(test)]
mod tests
{
    use crate::WriteMemory::*;

    #[test]
    fn WriteMemoryOnceComparison()
    {
        assert!(true);
    }

    /*#[test]
    fn WriteMemoryFreezeComparison()
    {
        FreezeMemory(1000);
        assert!(false);
    }*/
}