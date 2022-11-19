use std::str::FromStr;
use std::convert::From;
use std::fmt::Debug;
use std::mem::size_of;
use std::thread;
use std::sync::Arc;
use std::collections::HashSet;
use windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION;
use crate::Config::*;


pub struct PageCopy
{
    page_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION,
    memory: Vec<u8>
}

macro_rules! CompareMemoryValue
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
                println!("START-> Start: {} -- End: {} -- Search size: {} -- Buffer.len(): {}", start, end_index, private_region_size, buffer.len());


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

                println!("END-> Start: {} -- End: {} -- Search size: {} -- Buffer.len(): {}", start, end_index, private_region_size, buffer.len());


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


pub unsafe fn MatchMemory(filter_list: Vec<FilterOption>, target_value: String, buffer: Arc<Vec<u8>>, start: usize, private_region_size: usize) -> Vec<usize>
{
    let mut match_address: Vec<usize> = vec![];

    for filter in &filter_list
    {

        let result: Vec<usize> = match filter
        {
            FilterOption::U32 => CompareMemoryValue!( u32, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::U64 => CompareMemoryValue!( u64, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::I32 => CompareMemoryValue!( i32, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::I64 => CompareMemoryValue!( i64, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::F32 => CompareMemoryValue!( f32, target_value.clone(), &buffer, start, private_region_size ),
            FilterOption::F64 => CompareMemoryValue!( f64, target_value.clone(), &buffer, start, private_region_size ),
        };

        if result.len() != 0
        {
            match_address = [match_address, result].concat();
        }

        println!("Matches: {:?} -- Target: {}", match_address, target_value.clone());
    }

    return match_address;
}

pub fn MultithreadSearch(page_copy: Vec<u8>, thread_count: usize, filter_list: Vec<FilterOption>, target_value: String) -> HashSet<usize>
{
    //let mut size_per_thread: usize = 0;
    let size_per_thread = (page_copy.len() as f64 / thread_count as f64).ceil() as usize;

    let memory = Arc::new(page_copy);

    let mut thread_handles: Vec< thread::JoinHandle<Vec<usize>> > = Vec::with_capacity(0);

    // Start threads
    for thread_id in 0..thread_count
    {
        let arc_clone = Arc::clone(&memory);
        let filter_list_clone = filter_list.clone();
        let target_value_clone = target_value.clone();
        thread_handles.push(thread::spawn(move || unsafe{ MatchMemory(filter_list_clone, target_value_clone, arc_clone, thread_id*size_per_thread, size_per_thread) } ));
    }

    let mut final_result: HashSet<usize> = HashSet::new(); // Will help deduplicate results
    // Wait for all threads to finish and collect the results
    for thread in thread_handles.into_iter()
    {
        let results_array = thread.join().unwrap();

        for result in results_array
        {
            final_result.insert(result);
        }
        
    }

    return final_result;
}


// Unit test
#[cfg(test)]
mod tests
{
    use crate::ReadMemory::*;

    #[test]
    fn MemoryComparison()
    {
        let buffer: Vec<u8> = vec![15, 0, 0, 0];
        assert_eq!([0 as usize].to_vec(), CompareMemoryValue!(u32, "15".to_string(), &buffer, 0, buffer.len()));
    }

    #[test]
    fn EmptyMatchMemoryComparison()
    {
        let buffer: Vec<u8> = vec![15, 0, 0, 0];
        assert_eq!(true, CompareMemoryValue!(u32, "10".to_string(), &buffer, 0, buffer.len()).len() == 0);
    }

    #[test]
    fn FindMatch_RegularCase()
    {
        unsafe
        {
            let buffer: Arc<Vec<u8>> = Arc::new(vec![15, 0, 0, 0, 0, 0, 0, 0, 0]);
            let buffer_len = buffer.len();
            assert_eq!( [0].to_vec(), MatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 0, buffer_len ));
        }
    }

    #[test]
    fn FindMatch_RegularCaseMiddle()
    {
        unsafe
        {
            let buffer: Arc<Vec<u8>> = Arc::new(vec![0, 0, 0, 0, 0, 15, 0, 0, 0]);
            let buffer_len = buffer.len();
            assert_eq!( [5].to_vec(), MatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 5, buffer_len-5 ));
        }
    }

    #[test]
    fn FindMatch_ArraySmallerThanType()
    {
        unsafe
        {
            let buffer: Arc<Vec<u8>> = Arc::new(vec![15, 0, 0, 0]);
            let buffer_len = buffer.len();
            assert_eq!( true, MatchMemory([FilterOption::U64].to_vec(), "15".to_string(), buffer, 0, buffer_len ).len() == 0);
        }
    }

    #[test]
    fn FindMatch_Start_AllocatedSpaceSmallerThanType()
    {
        unsafe
        {
            // Consider 4 threads reading the buffer
            let buffer: Arc<Vec<u8>> = Arc::new(vec![15, 0, 0, 0, 0, 0, 0, 0]);
            assert_eq!( [0].to_vec(), MatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 0, 2 ));
        }
    }

    #[test]
    fn FindMatch_End_AllocatedSpaceSmallerThanType()
    {
        unsafe
        {
            // Consider 4 threads reading the buffer
            let buffer: Arc<Vec<u8>> = Arc::new(vec![0, 0, 0, 0, 15, 0, 0, 0]);
            assert_eq!( [4].to_vec(), MatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 4, 2 ));
        }
    }

    #[test]
    fn FindMatch_ThreadStartOutOfBounds()
    {
        unsafe
        {
            // Consider 10 threads reading the buffer and the space is ceil-rounded - thread 10
            let buffer: Arc<Vec<u8>> = Arc::new(vec![0, 0, 0, 0, 15, 0, 0, 0]);
            assert_eq!( true, MatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 10, 1 ).len() == 0);
        }
    }

    #[test]
    fn FindMatch_ThreadStartNotEnounghSpace()
    {
        unsafe
        {
            // Consider 10 threads reading the buffer and the space is ceil-rounded - thread 7
            let buffer: Arc<Vec<u8>> = Arc::new(vec![0, 0, 0, 0, 15, 0, 0, 0]);
            assert_eq!( true, MatchMemory([FilterOption::U32].to_vec(), "15".to_string(), buffer, 7, 1 ).len() == 0);
        }
    }

    #[test]
    fn FindMatch_MultiThread_RegularCase()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![15, 0, 0, 0, 0, 0, 0, 0, 0];
            assert!( MultithreadSearch(buffer, 6, [FilterOption::U32].to_vec(), "15".to_string()).contains(&0) );
        }
    }

    #[test]
    fn FindMatch_MultiThread_MiddleCase()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![0, 0, 0, 0, 15, 0, 0, 0, 0];
            assert!( MultithreadSearch(buffer, 6, [FilterOption::U32].to_vec(), "15".to_string()).contains(&4) );
        }
    }

    #[test]
    fn FindMatch_MultiThread_EndCase()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![0, 0, 0, 0, 0, 15, 0, 0, 0];
            assert!( MultithreadSearch(buffer, 6, [FilterOption::U32].to_vec(), "15".to_string()).contains(&5) );
        }
    }

    #[test]
    fn FindMatch_MultiThread_Intersection()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![0, 0, 15, 0, 0, 0, 0, 0];
            assert!( MultithreadSearch(buffer, 2, [FilterOption::U32].to_vec(), "15".to_string()).contains(&2) );
        }
    }
}
