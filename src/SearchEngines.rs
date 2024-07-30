use std::mem::size_of;

// A simple linear search algorithm
// It simply read byte per byte and check for equality
fn NaiveLinearSearch()
{

}

// A simple linear search algorithm
// It simply read byte per byte and check for equality
// Insted of accecpting a type, it accepts a [u8] pattern to search
fn NaiveLinearSearchPattern()
{

}


// This function is shared between the comparator and its filter
macro_rules! SelectCompareValues
{
    ($value_to_check: expr, $target_value: expr, $comparison_op: expr) =>
    {
        match $comparison_op.as_str()
        {
            "!=" => $value_to_check != $target_value,
            "==" => $value_to_check == $target_value,
            ">" => $value_to_check > $target_value,
            "<" => $value_to_check < $target_value,
            ">=" => $value_to_check >= $target_value,
            "<=" => $value_to_check <= $target_value,
            _ => panic!("No comparison operation!")
        };
    }
}

// It is similar to the naive linear search
// The difference is that is transforms the value and perform comparisions
// For example, it takes the buffer -> transforms it into a type (such as fp32) -> makes comparisons
// It iterates over a memory slice (not positions)
macro_rules! Comparator
{
    ($target_type:ty, $func_name: ident) =>
    {
        // The mem_region_slice holds a partial view of a bigger buffer and of the private segment (thread workload), it represents the slice of memory belonging to a specific region
        // It also prevents the function from reading from another region or private segment in the buffer
        // The slice start is necessary to ajust the results, so it returns an address relative to the memory region
        pub fn $func_name(mem_region_slice_view: &[u8], slice_view_start: usize, operations: Vec<(String, $target_type)>, match_buffer_size: usize) -> Vec<usize>
        {
            // Store all the results
            // Buffer sized if based on usize's size - how many addresses can we store?
            let mut match_addresses: Vec<usize> = Vec::with_capacity(match_buffer_size * size_of::<usize>());

            // Iterate over positions
            // Note that safety must be guaranteed by the function itself (see workload partitioning - slice_view)
            // -1 is necessary to avoid out of bounds read (remember the non inclusive end!)
            // u8: [0..10] -> 10 - (1-1) = 10 | stops at 9 and reads only 9
            // u32: [0..10] -> 10 - (4-1) = 7 | stops at 6 and reads 6,7,8,9
            // u64: [0..10] -> 10 - (8-1) = 3 | stops at 2 and reads 2,3,4,5,6,7,8,9
            let end: usize = mem_region_slice_view.len() - (size_of::<$target_type>() - 1);
            for current_pos in 0..end
            {
                // Assume that it will match
                let mut cmp_result: bool = true;

                // Transform the memory into a type
                // This transformation (associated function) cannot accept a generic, so I will use a macro to bypass it - a trait might solve it
                let value_to_check: $target_type = <$target_type>::from_ne_bytes( mem_region_slice_view[current_pos..(current_pos+size_of::<$target_type>())].try_into().unwrap() );

                // What kind of comparison should we make?
                // If there is less comparisons than what is supported (2), it will assume the it was true
                // This allows to make make single checks like: >100 (without having to specify another check)
                for op_idx in 0..operations.len()
                {
                    // Unpack the operation
                    let comparison_op: &String = &operations[op_idx].0;
                    let target_value: $target_type = operations[op_idx].1;
                
                    let result = SelectCompareValues!(value_to_check, target_value, comparison_op);

                    // The value does not satify the constraints
                    if result == false
                    {
                        // Alert the outer loop
                        cmp_result = result;

                        // There is no need to continue the verification, continue to the next value
                        break;
                    }
                }

                // The match was successful
                if cmp_result == true
                {
                    // Store the match and ajust its address to be relative to the region, not the slice
                    match_addresses.push(current_pos + slice_view_start);
                }
            }
            return match_addresses;
        }
    }
}

// Declaring the supported types for other modules to simply use a function
// It is important to be a function, so all of the uses point to the same code, allowing the code cache to be better used
// Signed int
Comparator!(i8, LinearSearch_Comparator_i8);
Comparator!(i16, LinearSearch_Comparator_i16);
Comparator!(i32, LinearSearch_Comparator_i32);
Comparator!(i64, LinearSearch_Comparator_i64);
Comparator!(i128, LinearSearch_Comparator_i128);

// Unsigned int
Comparator!(u8, LinearSearch_Comparator_u8);
Comparator!(u16, LinearSearch_Comparator_u16);
Comparator!(u32, LinearSearch_Comparator_u32);
Comparator!(u64, LinearSearch_Comparator_u64);
Comparator!(u128, LinearSearch_Comparator_u128);

// Floating point
// There is no native 8 or 16 bits floating point type
Comparator!(f32, LinearSearch_Comparator_f32);
Comparator!(f64, LinearSearch_Comparator_f64);


// It filters the matches found in the comparator search
macro_rules! ComparatorFilter
{
    ($target_type:ty, $func_name: ident) =>
    {
        // The mem_region_slice holds a partial view of a bigger buffer and of the private segment (thread workload), it represents the slice of memory belonging to a specific region
        // It also prevents the function from reading from another region or private segment in the buffer
        // The slice start is necessary to ajust the results, so it returns an address relative to the memory region
        pub fn $func_name(mem_region_slice_view: &[u8], slice_view_start: usize, operations: Vec<(String, $target_type)>, match_buffer_size: usize, previous_matches: Vec<usize>) -> Vec<usize>
        {
            // Store all the results
            // Buffer sized if based on usize's size - how many addresses can we store?
            let mut match_addresses: Vec<usize> = Vec::with_capacity(match_buffer_size * size_of::<usize>());

            // Iterate pervious matches
            // Note that the slice must contain the searched matches
            let end: usize = mem_region_slice_view.len() - (size_of::<$target_type>() - 1);
            for prev_match in previous_matches
            {
                // The match address is relative to the memory region
                // So ajust it to the slice
                let slice_relative_prev_match: usize = prev_match - slice_view_start;

                // Assume that it will match
                let mut cmp_result: bool = true;

                // Transform the memory into a type
                // This transformation (associated function) cannot accept a generic, so I will use a macro to bypass it - a trait might solve it
                let value_to_check: $target_type = <$target_type>::from_ne_bytes( mem_region_slice_view[slice_relative_prev_match..(slice_relative_prev_match+size_of::<$target_type>())].try_into().unwrap() );

                // What kind of comparison should we make?
                // If there is less comparisons than what is supported (2), it will assume the it was true
                // This allows to make make single checks like: >100 (without having to specify another check)
                for op_idx in 0..operations.len()
                {
                    // Unpack the operation
                    let comparison_op: &String = &operations[op_idx].0;
                    let target_value: $target_type = operations[op_idx].1;
                
                    let result = SelectCompareValues!(value_to_check, target_value, comparison_op);

                    // The value does not satify the constraints
                    if result == false
                    {
                        // Alert the outer loop
                        cmp_result = result;

                        // There is no need to continue the verification, continue to the next value
                        break;
                    }
                }

                // The match was successful
                if cmp_result == true
                {
                    // Store the match
                    // We simply store the original value, since it already represents the address relative to the memory regiion
                    match_addresses.push(prev_match);
                }
            }
            return match_addresses;
        }
    }
}

// Declaring the supported types for other modules to simply use a function
// It is important to be a function, so all of the uses point to the same code, allowing the code cache to be better used
// Signed int
ComparatorFilter!(i8, LinearSearch_ComparatorFilter_i8);
ComparatorFilter!(i16, LinearSearch_ComparatorFilter_i16);
ComparatorFilter!(i32, LinearSearch_ComparatorFilter_i32);
ComparatorFilter!(i64, LinearSearch_ComparatorFilter_i64);
ComparatorFilter!(i128, LinearSearch_ComparatorFilter_i128);

// Unsigned int
ComparatorFilter!(u8, LinearSearch_ComparatorFilter_u8);
ComparatorFilter!(u16, LinearSearch_ComparatorFilter_u16);
ComparatorFilter!(u32, LinearSearch_ComparatorFilter_u32);
ComparatorFilter!(u64, LinearSearch_ComparatorFilter_u64);
ComparatorFilter!(u128, LinearSearch_ComparatorFilter_u128);

// Floating point
// There is no native 8 or 16 bits floating point type
ComparatorFilter!(f32, LinearSearch_ComparatorFilter_f32);
ComparatorFilter!(f64, LinearSearch_ComparatorFilter_f64);


#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::SearchEngines::*;
    use std::sync::Arc;

    #[test]
    fn TestSearchEngines_Comparator()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: f32 = 15.0;
        let needle_size_bytes: usize = size_of::<f32>();
        let mut insert_pos = 25;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, f32)> = vec![(">".to_string(), 10.0 as f32), ("<".to_string(), 20.0 as f32)];

        // Even if you use arc, you can still slice it
        let arc_buffer = Arc::new(buffer);

        let result = LinearSearch_Comparator_f32(&arc_buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorEqual()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u8>();
        let mut insert_pos = 25;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![("==".to_string(), 15)];

        let result = LinearSearch_Comparator_u8(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorUnequal()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u8>();
        let mut insert_pos = 25;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![("!=".to_string(), 0)];

        let result = LinearSearch_Comparator_u8(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorHigherThan()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u8>();
        let mut insert_pos = 25;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![(">".to_string(), 14)];

        let result = LinearSearch_Comparator_u8(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorHigherThanOrEqual()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let mut needle: u8 = 25;
        let needle_size_bytes: usize = size_of::<u8>();
        let mut insert_pos = 25;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        needle = 15;
        insert_pos = 30;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![(">=".to_string(), 15)];

        let result = LinearSearch_Comparator_u8(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25, 30];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorLowerThan()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![10; buffer_size];

        // Create and insert needle
        let needle: u8 = 9;
        let needle_size_bytes: usize = size_of::<u8>();
        let mut insert_pos = 25;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![("<".to_string(), 10)];

        let result = LinearSearch_Comparator_u8(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorLowerThanOrEqual()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![10; buffer_size];

        // Create and insert needle
        let mut needle: u8 = 9;
        let needle_size_bytes: usize = size_of::<u8>();
        let mut insert_pos = 25;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        needle = 5;
        insert_pos = 30;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![("<=".to_string(), 9)];

        let result = LinearSearch_Comparator_u8(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25, 30];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorRange()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u8>();
        let mut insert_pos = 25;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![(">".to_string(), 10), ("<".to_string(), 20)];

        let result = LinearSearch_Comparator_u8(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorOverlapping()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle_size_bytes: usize = size_of::<u32>();
        buffer[25] = 1;
        buffer[26] = 1;
        buffer[27] = 1;

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u16)> = vec![("==".to_string(), 257)];

        let result = LinearSearch_Comparator_u16(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25, 26];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorEndTarget()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u32 = 15;
        let needle_size_bytes: usize = size_of::<u32>();
        let mut insert_pos = 46;
        for needle_byte in needle.to_ne_bytes()
        {
            buffer[insert_pos] = needle_byte;
            insert_pos = insert_pos + 1;
        }

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u32)> = vec![("==".to_string(), 15)];

        let result = LinearSearch_Comparator_u32(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![46];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorEndTargetSingleByte()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u32>();
        let mut insert_pos = 49;
        buffer[insert_pos] = needle;

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![("==".to_string(), 15)];

        let result = LinearSearch_Comparator_u8(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![49];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorEndTargetEmpty()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u32>();
        let mut insert_pos = 47;

        // It shouldn't be able to read this needle
        buffer[insert_pos] = needle;

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u32)> = vec![("==".to_string(), 15)];

        let result = LinearSearch_Comparator_u32(&buffer[0..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        let expected: Vec<usize> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorRelativeAddress()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u32>();
        let mut insert_pos = 25;

        // It shouldn't be able to read this needle
        buffer[insert_pos] = needle;

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 25;
        let operations: Vec<(String, u32)> = vec![("==".to_string(), 15)];

        // Onlt share part of the buffer and see if it corrects the output
        let result = LinearSearch_Comparator_u32(&buffer[start..buffer_size], start, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorFilter()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u32>();
        let mut insert_pos = 30;

        buffer[insert_pos] = needle;

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![("==".to_string(), 15)];

        // Assume the it had a match at 25 previously
        // Assume that the needle moved
        let previous_matches: Vec<usize> = vec![25];

        // Onlt share part of the buffer and see if it corrects the output
        let result = LinearSearch_ComparatorFilter_u8(&buffer[start..buffer_size], start, operations, 1000, previous_matches);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorFilterSomethingRemains()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u32>();
        let mut insert_pos = 25;

        buffer[insert_pos] = needle;

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        let start: usize = 0;
        let operations: Vec<(String, u8)> = vec![("==".to_string(), 15)];

        // Assume the it had a match at 25, 30 and 45 previously
        // Assume that the needle is actually 25 and the buffer changed to represent this
        let previous_matches: Vec<usize> = vec![25, 30, 45];

        // Onlt share part of the buffer and see if it corrects the output
        let result = LinearSearch_ComparatorFilter_u8(&buffer[start..buffer_size], start, operations, 1000, previous_matches);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }

    #[test]
    fn TestSearchEngines_ComparatorFilterAjustAddress()
    {
        let buffer_size = 50;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        // Create and insert needle
        let needle: u8 = 15;
        let needle_size_bytes: usize = size_of::<u32>();

        buffer[25] = needle;
        buffer[30] = needle;

        // Print the current state of the buffer
        println!("Buffer: {:?}", buffer);

        // The thread only gets half of the buffer
        let start: usize = 25;
        let operations: Vec<(String, u8)> = vec![("==".to_string(), 15)];

        // Assume the it had a match at 25, 30 and 45 previously
        // Assume that the needle is actually 25 and the buffer changed to represent this
        let previous_matches: Vec<usize> = vec![25, 30, 45];

        // Onlt share part of the buffer and see if it corrects the output
        let result = LinearSearch_ComparatorFilter_u8(&buffer[start..buffer_size], start, operations, 1000, previous_matches);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25, 30];
        assert_eq!(result, expected);
    }
}