use std::str::FromStr;
use std::convert::From;
use std::fmt::Debug;
use std::mem::size_of;
use std::thread;
use std::sync::Arc;
use std::collections::HashSet;
use std::os::raw::c_void;
use std::mem::take;
use windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION;
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Foundation::GetLastError;
use crate::Config::*;


pub struct PageCopy
{
    page_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION,
    memory: Vec<u8>
}

// All the addesses for a given type -> there is a u32 at address 15
#[derive(Debug)]
pub struct AddressMatch
{
    U32: Vec<usize>,
    U64: Vec<usize>,
    I32: Vec<usize>,
    I64: Vec<usize>,
    F32: Vec<usize>,
    F64: Vec<usize>
}

impl AddressMatch
{
    pub fn new() -> Self
    {
        return AddressMatch
        {
            U32: vec![],
            U64: vec![],
            I32: vec![],
            I64: vec![],
            F32: vec![],
            F64: vec![]
        };
    }

    pub fn is_empty(&self) -> bool
    {
        if self.U32.len() + self.U64.len() + self.I32.len() + self.I64.len() + self.F32.len() + self.F64.len() == 0
        {
            return true;
        }
        return false;
    }
}

pub struct MemoryMatches
{
    page_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION,
    matches: AddressMatch,
}

impl MemoryMatches
{
    pub fn new(page: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION, search_results: AddressMatch) -> Self
    {
        return MemoryMatches
        {
            page_info: page,
            matches: search_results
        };
    }
}

macro_rules! InitialCompareMemoryValue
{
    ($target_type:ty, $target_value:expr, $buffer:expr, $start:expr, $size:expr) =>
    {
        // Buffer.len() == total space
        // private_region_size == the amount of bytes each thread will search
        |target_value: String, buffer: &[u8], start: usize, private_region_size: usize| -> Vec<usize>
        {
            // Buffer doesn't have enough space to do type conversion
            // Thread start doesn't have enough space left
            if(buffer.len() >= size_of::<$target_type>() && start + size_of::<$target_type>() <= buffer.len() )
            {
                let mut match_address: Vec<usize> = [].to_vec();

                let mut end_index = start + private_region_size;
                // Integer (under)overflow protection
                end_index=if(end_index < size_of::<$target_type>()) { 0 } else{end_index-size_of::<$target_type>()};

                let extension_size = size_of::<$target_type>() - 1; // How much memory we can overlap for search, so intersections between threads are also considered
                let parsed_target_value: $target_type = target_value.parse::<$target_type>().unwrap();

                // CHECKS
                // The loop must only contain a valid range
                ////println!("START-> Start: {} -- End: {} -- Search size: {} -- Buffer.len(): {}", start, end_index, private_region_size, buffer.len());

                // In case the allocated size per thread (!= buffer.len()) being smaller than data type
                if end_index < start
                {
                    end_index = start;
                }

                // we haven't reached the buffer limit, so we can add an extension
                if end_index + size_of::<$target_type>() < buffer.len()
                {   
                    end_index += extension_size;
                }

                // End_index goes out of bounds, this fixes it
                // What does happen when it doesn't have enough space? The first condition will prevent this
                if end_index + size_of::<$target_type>() >= buffer.len()
                {
                    end_index = buffer.len() - size_of::<$target_type>();
                }

                ////println!("END-> Start: {} -- End: {} -- Search size: {} -- Buffer.len(): {}", start, end_index, private_region_size, buffer.len());

                // Since the biggest overhead comes from the loop, all values and checks are pre-computed
                // and with this approach, we avoid the need to do bounds-check in every iteration
                // +1 (inclusive end)
                for index in start..end_index+1
                {
                    let process_memory_value = <$target_type>::from_le_bytes( buffer[index..( index+size_of::<$target_type>() )].try_into().unwrap() );

                    if parsed_target_value == process_memory_value
                    {
                        match_address.push(index);
                    }
                }
                return match_address;
            }

            else
            {
                return vec![];
            }

        }($target_value, $buffer, $start, $size)
    }
}


pub unsafe fn InitialMatchMemory(filter_list: Vec<FilterOption>, target_value: String, buffer: Arc<Vec<u8>>, start: usize, private_region_size: usize) -> AddressMatch
{
    let mut match_address = AddressMatch::new();

    for filter in &filter_list
    {

        let result: Vec<usize> = match filter
        {
            FilterOption::U32 => InitialCompareMemoryValue!( u32, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::U64 => InitialCompareMemoryValue!( u64, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::I32 => InitialCompareMemoryValue!( i32, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::I64 => InitialCompareMemoryValue!( i64, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::F32 => InitialCompareMemoryValue!( f32, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::F64 => InitialCompareMemoryValue!( f64, target_value.clone(), &buffer, start, private_region_size ),
        };

        // Put the results in the right field
        if result.len() != 0
        {
            match filter
            {
                FilterOption::U32 => match_address.U32.extend(result.iter().cloned()),
                FilterOption::U64 => match_address.U64.extend(result.iter().cloned()),
                FilterOption::I32 => match_address.I32.extend(result.iter().cloned()),
                FilterOption::I64 => match_address.I64.extend(result.iter().cloned()),
                FilterOption::F32 => match_address.F32.extend(result.iter().cloned()),
                FilterOption::F64 => match_address.F64.extend(result.iter().cloned()),
            }
        }
        ////println!("Matches: {:?} -- Target: {}", match_address, target_value.clone());
    }
    return match_address;
}

pub fn InitialMultithreadSearch(page_copy: Vec<u8>, thread_count: usize, filter_list: Vec<FilterOption>, target_value: String) -> AddressMatch
{
    //let mut size_per_thread: usize = 0;
    let size_per_thread = (page_copy.len() as f64 / thread_count as f64).ceil() as usize;

    let memory = Arc::new(page_copy);

    let mut thread_handles: Vec< thread::JoinHandle<AddressMatch> > = Vec::with_capacity(0);

    // Start threads
    for thread_id in 0..thread_count
    {
        let arc_clone = Arc::clone(&memory);
        let filter_list_clone = filter_list.clone();
        let target_value_clone = target_value.clone();
        thread_handles.push(thread::spawn(move || unsafe{ InitialMatchMemory(filter_list_clone, target_value_clone, arc_clone, thread_id*size_per_thread, size_per_thread) } ));
    }

    let mut final_result = AddressMatch::new();
    // Wait for all threads to finish and collect the results
    for thread in thread_handles.into_iter()
    {
        let result = thread.join().unwrap();

        final_result.U32.extend(result.U32.iter().cloned());
        final_result.U64.extend(result.U64.iter().cloned());
        final_result.I32.extend(result.I32.iter().cloned());
        final_result.I64.extend(result.I64.iter().cloned());
        final_result.F64.extend(result.F64.iter().cloned());
        final_result.F32.extend(result.F32.iter().cloned());

        // Sort and deduplicate results
        final_result.U32.sort();
        final_result.U32.dedup();

        final_result.U64.sort();
        final_result.U64.dedup();

        final_result.I32.sort();
        final_result.I32.dedup();

        final_result.I64.sort();
        final_result.I64.dedup();

        final_result.F32.sort();
        final_result.F32.dedup();

        final_result.F64.sort();
        final_result.F64.dedup();
    }
    return final_result;
}

fn GetAllWritablePagesInfo(mut process: windows_sys::Win32::Foundation::HANDLE) -> Vec<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION>
{
    let mut readwrite_memory: Vec<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION> = vec![];
    unsafe
    {
        let mut p: *mut std::os::raw::c_void = std::ptr::null_mut();

        let mut memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: 0,
            Type: 0,
        };
        
        // Loop over successful attempts
        while windows_sys::Win32::System::Memory::VirtualQueryEx(process, p, &mut memory_info, std::mem::size_of::<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION>()) == std::mem::size_of::<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION>()
        {
            // This will remove some bits, thus avoiding some problems in the comparison below
            memory_info.AllocationProtect &= !(windows_sys::Win32::System::Memory::PAGE_GUARD | windows_sys::Win32::System::Memory::PAGE_NOCACHE);

            // Commitable (memory effectively used), writable and readable pages only
            if (memory_info.AllocationProtect == windows_sys::Win32::System::Memory::PAGE_READWRITE || memory_info.AllocationProtect == windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE) && (memory_info.State == windows_sys::Win32::System::Memory::MEM_COMMIT)
            {
                readwrite_memory.push(memory_info);
            }

            p = memory_info.BaseAddress; // Align the addresses
            p = p.add(memory_info.RegionSize);
        }
    }
    return readwrite_memory;
}

pub fn SearchProcessMemory_Initial(filter_list: Vec<FilterOption>, thread_count: usize, target_value: String, process_handle: HANDLE) -> Vec<MemoryMatches>
{
    let all_memory_sections_info = GetAllWritablePagesInfo(process_handle);
    let mut all_pages_matches: Vec<MemoryMatches> = vec![]; 

    for memory_section_info in all_memory_sections_info
    {
        let mut memory_region_buffer: Vec<u8> = Vec::with_capacity(memory_section_info.RegionSize);
        unsafe {memory_region_buffer.set_len( memory_region_buffer.capacity() );}
        // Alternative - Could be slower, since it initializes the memory to zero
        //let mut memory_region_buffer: Vec<u8> = vec![0; memory_section_info.RegionSize];

        let mut transfered_bytes: usize = 0;
        let mut transfered_bytes_ptr: *mut usize = &mut transfered_bytes;

        unsafe
        {
            // Get the copy of the memory region
            let success_code = ReadProcessMemory(process_handle, memory_section_info.BaseAddress, memory_region_buffer.as_mut_ptr() as *mut c_void, memory_section_info.RegionSize, transfered_bytes_ptr);

            println!("transfered_bytes: {} -- Base Address: {} -- Region Size: {} -- Buffer.len: {} -- Code: {} -- GetLastError: {}", transfered_bytes, memory_section_info.BaseAddress as usize, memory_section_info.RegionSize, memory_region_buffer.len(), success_code, GetLastError());

            // Search for successful attempts only
            if success_code != 0
            {
                let search = InitialMultithreadSearch(memory_region_buffer, thread_count, filter_list.clone(), target_value.clone());

                println!("Search result: {:?}\n", search);

                if search.is_empty() != true
                {
                    all_pages_matches.push( MemoryMatches::new(memory_section_info, search) );
                }
            }
        }
    }
    return all_pages_matches;
}

macro_rules! ComparePreviousMatch
{
    ($target_type:ty, $target_value:expr, $buffer:expr, $matches:expr, $thread_count:expr) =>
    {
        |target_value: String, buffer_arc: Arc<Vec<u8>>, page_matches: Vec<usize>, thread_count: usize| -> Vec<usize>
        {
            let addresses_per_thread = (page_matches.len() as f64 / thread_count as f64).ceil() as usize;
            let mut thread_handles: Vec< thread::JoinHandle<Vec<usize>> > = Vec::with_capacity(0);
            let parsed_target_value: $target_type = target_value.parse::<$target_type>().unwrap();

            let page_matches_arc = Arc::new(page_matches);

            // Start threads
            for thread_id in 0..thread_count
            {
                let matches_arc_clone = Arc::clone(&page_matches_arc);
                let buffer_arc_clone = Arc::clone(&buffer_arc);
                let target_value_clone = target_value.clone();
                let addresses_per_thread = addresses_per_thread.clone();

                thread_handles.push(thread::spawn(move ||
                {
                    let mut thread_result: Vec<usize> = vec![];

                    let start = thread_id*addresses_per_thread;
                    let mut end = start+addresses_per_thread;

                    // Thread starts out of bounds
                    if start > matches_arc_clone.len()
                    {
                        return thread_result;
                    }

                    // Thread reads more than what it is supposse to read
                    if end > matches_arc_clone.len()
                    {
                        end = matches_arc_clone.len();
                    }

                    //println!("ThreadID: {} -- Adresses per thread: {} -- Start: {} -- End: {} -- Matches: {}", thread_id, addresses_per_thread, start, end, matches_arc_clone.len());
                    for address in &matches_arc_clone[start..end]
                    {
                        let process_memory_value = <$target_type>::from_le_bytes( buffer_arc_clone[*address..( *address+size_of::<$target_type>() )].try_into().unwrap() );

                        if parsed_target_value == process_memory_value
                        {
                            thread_result.push(*address);
                        }
                    }

                    return thread_result;
                }));
            }

            let mut final_result: Vec<usize> = vec![];
            // Wait for all threads to finish and collect the results
            for thread in thread_handles.into_iter()
            {
                let result = thread.join().unwrap();

                final_result.extend(result.iter().cloned());

                // Sort and deduplicate results
                final_result.sort();
            }
            return final_result;
        }($target_value.clone(), $buffer, $matches, $thread_count)
    }
}


pub fn FilterMatches(search_results: Vec<MemoryMatches>, filter_list: Vec<FilterOption>, thread_count: usize, target_value: String, process_handle: HANDLE) -> Vec<MemoryMatches>
{
    let mut search_previous_results: Vec<MemoryMatches> = vec![];

    for mut page_section in search_results
    {
        // Get memory copy
        let mut memory_region_copy: Vec<u8> = Vec::with_capacity(page_section.page_info.RegionSize);
        unsafe {memory_region_copy.set_len( memory_region_copy.capacity() );}
        let success_code = unsafe{ ReadProcessMemory(process_handle, page_section.page_info.BaseAddress, memory_region_copy.as_mut_ptr() as *mut c_void, page_section.page_info.RegionSize, std::ptr::null_mut()) };

        //Arc for the buffers
        let memory_region_copy_arc = Arc::new(memory_region_copy);

        // Search any match in previous results
        let mut match_address = AddressMatch::new();
        let mut match_count = 0;
        for filter in filter_list.iter()
        {
            // Check if the previous addresses match the new target value 
            let result: Vec<usize> = match filter
            {
                FilterOption::U32 => ComparePreviousMatch!( u32, target_value.clone(), Arc::clone(&memory_region_copy_arc), std::mem::take(&mut page_section.matches.U32), thread_count),
                FilterOption::U64 => ComparePreviousMatch!( u64, target_value.clone(), Arc::clone(&memory_region_copy_arc), std::mem::take(&mut page_section.matches.U64), thread_count),
                FilterOption::I32 => ComparePreviousMatch!( i32, target_value.clone(), Arc::clone(&memory_region_copy_arc), std::mem::take(&mut page_section.matches.I32), thread_count),
                FilterOption::I64 => ComparePreviousMatch!( i64, target_value.clone(), Arc::clone(&memory_region_copy_arc), std::mem::take(&mut page_section.matches.I64), thread_count),
                FilterOption::F32 => ComparePreviousMatch!( f32, target_value.clone(), Arc::clone(&memory_region_copy_arc), std::mem::take(&mut page_section.matches.F32), thread_count),
                FilterOption::F64 => ComparePreviousMatch!( f64, target_value.clone(), Arc::clone(&memory_region_copy_arc), std::mem::take(&mut page_section.matches.F64), thread_count),
            };

            match_count += result.len();

            // Put the results in the right field
            if result.len() != 0
            {
                match filter
                {
                    FilterOption::U32 => match_address.U32.extend(result.iter().cloned()),
                    FilterOption::U64 => match_address.U64.extend(result.iter().cloned()),
                    FilterOption::I32 => match_address.I32.extend(result.iter().cloned()),
                    FilterOption::I64 => match_address.I64.extend(result.iter().cloned()),
                    FilterOption::F32 => match_address.F32.extend(result.iter().cloned()),
                    FilterOption::F64 => match_address.F64.extend(result.iter().cloned()),
                }
            }
        }

        // OK, found something -> Store the page info and the addresses
        if match_count != 0
        {
            search_previous_results.push( MemoryMatches::new( page_section.page_info, match_address) );
        }
    }
    return search_previous_results;
}

// Unit test
#[cfg(test)]
mod tests
{
    use crate::ReadMemory::*;
    use std::io;

    #[test]
    fn MemoryComparison()
    {
        let buffer: Vec<u8> = vec![15, 0, 0, 0];
        assert_eq!([0 as usize].to_vec(), InitialCompareMemoryValue!(u32, "15".to_string(), &buffer, 0, buffer.len()));
    }

    #[test]
    fn EmptyMatchMemoryComparison()
    {
        let buffer: Vec<u8> = vec![15, 0, 0, 0];
        assert_eq!(true, InitialCompareMemoryValue!(u32, "10".to_string(), &buffer, 0, buffer.len()).len() == 0);
    }

    #[test]
    fn FindMatch_RegularCase()
    {
        unsafe
        {
            let buffer: Arc<Vec<u8>> = Arc::new(vec![15, 0, 0, 0, 0, 0, 0, 0, 0]);
            let buffer_len = buffer.len();
            assert_eq!( [0].to_vec(), InitialMatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 0, buffer_len ).U32);
        }
    }

    #[test]
    fn FindMatch_RegularCaseMiddle()
    {
        unsafe
        {
            let buffer: Arc<Vec<u8>> = Arc::new(vec![0, 0, 0, 0, 0, 15, 0, 0, 0]);
            let buffer_len = buffer.len();
            assert_eq!( [5].to_vec(), InitialMatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 5, buffer_len-5 ).U32);
        }
    }

    #[test]
    fn FindMatch_ArraySmallerThanType()
    {
        unsafe
        {
            let buffer: Arc<Vec<u8>> = Arc::new(vec![15, 0, 0, 0]);
            let buffer_len = buffer.len();
            assert_eq!( true, InitialMatchMemory([FilterOption::U64].to_vec(), "15".to_string(), buffer, 0, buffer_len ).U64.len() == 0);
        }
    }

    #[test]
    fn FindMatch_Start_AllocatedSpaceSmallerThanType()
    {
        unsafe
        {
            // Consider 4 threads reading the buffer
            let buffer: Arc<Vec<u8>> = Arc::new(vec![15, 0, 0, 0, 0, 0, 0, 0]);
            assert_eq!( [0].to_vec(), InitialMatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 0, 2 ).U32);
        }
    }

    #[test]
    fn FindMatch_End_AllocatedSpaceSmallerThanType()
    {
        unsafe
        {
            // Consider 4 threads reading the buffer
            let buffer: Arc<Vec<u8>> = Arc::new(vec![0, 0, 0, 0, 15, 0, 0, 0]);
            assert_eq!( [4].to_vec(), InitialMatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 4, 2 ).U32);
        }
    }

    #[test]
    fn FindMatch_ThreadStartOutOfBounds()
    {
        unsafe
        {
            // Consider 10 threads reading the buffer and the space is ceil-rounded - thread 10
            let buffer: Arc<Vec<u8>> = Arc::new(vec![0, 0, 0, 0, 15, 0, 0, 0]);
            assert_eq!( true, InitialMatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 10, 1 ).U32.len() == 0);
        }
    }

    #[test]
    fn FindMatch_ThreadStartNotEnounghSpace()
    {
        unsafe
        {
            // Consider 10 threads reading the buffer and the space is ceil-rounded - thread 7
            let buffer: Arc<Vec<u8>> = Arc::new(vec![0, 0, 0, 0, 15, 0, 0, 0]);
            assert_eq!( true, InitialMatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 7, 1 ).U32.len() == 0);
        }
    }
    
    #[test]
    fn FindMatch_MultiThread_RegularCase()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![15, 0, 0, 0, 0, 0, 0, 0, 0];
            assert!( InitialMultithreadSearch(buffer, 6, [FilterOption::U32].to_vec(), "15".to_string()).U32.contains(&0) );
        }
    }

    #[test]
    fn FindMatch_MultiThread_MiddleCase()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![0, 0, 0, 0, 15, 0, 0, 0, 0];
            assert!( InitialMultithreadSearch(buffer, 6, [FilterOption::U32].to_vec(), "15".to_string()).U32.contains(&4) );
        }
    }

    #[test]
    fn FindMatch_MultiThread_EndCase()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![0, 0, 0, 0, 0, 15, 0, 0, 0];
            assert!( InitialMultithreadSearch(buffer, 6, [FilterOption::U32].to_vec(), "15".to_string()).U32.contains(&5) );
        }
    }

    #[test]
    fn FindMatch_MultiThread_Intersection()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![0, 0, 15, 0, 0, 0, 0, 0];
            assert!( InitialMultithreadSearch(buffer, 2, [FilterOption::U32].to_vec(), "15".to_string()).U32.contains(&2) );
        }
    }


    #[test]
    fn SearchProcessTest()
    {
        unsafe
        {
            let process_handle: windows_sys::Win32::Foundation::HANDLE = windows_sys::Win32::System::Threading::OpenProcess(windows_sys::Win32::System::Threading::PROCESS_QUERY_INFORMATION |
                windows_sys::Win32::System::Threading::PROCESS_VM_OPERATION |
                windows_sys::Win32::System::Threading::PROCESS_VM_READ |
                windows_sys::Win32::System::Threading::PROCESS_VM_WRITE,
                0, // False
                11984); // Hardcoded process id

            let mut result = SearchProcessMemory_Initial(vec![FilterOption::U32], 6, "4".to_string(), process_handle);
            println!("Page matches: {}", &result.len());
            
            for i in 5..9
            {
                let mut guess = String::new();
                io::stdin().read_line(&mut guess).expect("failed to readline");

                println!("{guess}");

                result = FilterMatches(result, vec![FilterOption::U32], 6, i.to_string(), process_handle);
                println!("Matches: {}", &result.len());
            }

            windows_sys::Win32::Foundation::CloseHandle(process_handle);

            assert!(false);
        }
    }
}
