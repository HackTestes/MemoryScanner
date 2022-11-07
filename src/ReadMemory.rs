use std::str::FromStr;
use std::convert::From;
use std::fmt::Debug;
use std::mem::size_of;
use crate::Config::*;



macro_rules! CompareMemoryValue
{
    ($target_type:ty, $target_value:expr, $buffer:expr, $start:expr, $end:expr) =>
    {
        |target_value: String, buffer: &[u8], start: usize, private_region_size: usize| -> Vec<usize>
        {
            let mut match_address: Vec<usize> = [].to_vec();
            let mut end_index = private_region_size - size_of::<$target_type>();
            let extension_size = size_of::<$target_type>() - 1; // How much memory we can overlap for search, so intersections between threads are also considered

            let parsed_target_value: $target_type = target_value.parse::<$target_type>().unwrap();

            // == 0 -> we reached the buffer limit
            if (end_index + size_of::<$target_type>() - buffer.len()) != 0
            {
                end_index += extension_size;
            }

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

        }($target_value, $buffer, $start, $end)
    }
}



pub unsafe fn MatchMemory(filter_list: Vec<FilterOption>, target_value: String, buffer: &Vec<u8>, start: usize, private_region_size: usize) -> Vec<usize>
{
    let mut match_address: Vec<usize> = [].to_vec();

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
            _=> [].to_vec(),
        };

        if result.len() != 0
        {
            match_address = [match_address, result].concat();
        }

        println!("Matches: {:?} -- Target: {}", match_address, target_value.clone());
    }

    return match_address;
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
    fn MatchTest()
    {
        unsafe
        {
            let buffer: Vec<u8> = vec![15, 0, 0, 0, 0, 0, 0, 0, 0];
            assert_eq!( [0].to_vec(), MatchMemory([FilterOption::U32].to_vec(), "15".to_string(), &buffer, 0, buffer.len() ));
        }
    }
}