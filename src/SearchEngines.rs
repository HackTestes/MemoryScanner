use std::sync::Arc;
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

// It is similar to the naive linear search
// The difference is that is transforms the value and perform comparisions
// For example, it takes the buffer -> transforms it into a type (such as fp32) -> makes comparisons
// It iterates over positions (not a memory slice)
macro_rules! Comparator
{
    ($target_type:ty, $func_name: ident) =>
    {
        fn $func_name(buffer: Arc<Vec<u8>>, start: usize, end: usize, operations: Vec<(String, $target_type)>, match_buffer_size: usize) -> Vec<usize>
        {
            // Store all the results
            // Buffer sized if based on usize's size - how many addresses can we store?
            let mut match_addresses: Vec<usize> = Vec::with_capacity(match_buffer_size * size_of::<usize>());

            // Iterate over positions
            // Note that safety must be guaranteed by whatever is providing the range (see workload partitioning)
            for current_pos in start..end
            {
                // Assume that it will match
                let mut cmp_result: bool = true;

                // Transform the memory into a type
                // This transformation (associated function) cannot accept a generic, so I will use a macro to bypass it - a trait might solve it
                let value_to_check: $target_type = <$target_type>::from_ne_bytes( buffer[current_pos..(current_pos+size_of::<$target_type>())].try_into().unwrap() );

                // What kind of comparison should we make?
                // If there is less comparisons than what is supported (2), it will assume the it was true
                // This allows to make make single checks like: >100 (without having to specify another check)
                for op_idx in 0..operations.len()
                {
                    // Unpack the operation
                    let comparison_op: &String = &operations[op_idx].0;
                    let target_value: $target_type = operations[op_idx].1;
                
                    let result = match comparison_op.as_str()
                    {
                        "==" => value_to_check == target_value,
                        ">" => value_to_check > target_value,
                        "<" => value_to_check < target_value,
                        ">=" => value_to_check >= target_value,
                        "<=" => value_to_check <= target_value,
                        _ => panic!("No comparison operation!")
                    };

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
                    match_addresses.push(current_pos);
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

#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::SearchEngines::*;
    use std::sync::Arc;

    #[test]
    fn TestSearchEngines_Comparator()
    {
        let mut buffer: Vec<u8> = vec![0; 50];

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
        let end: usize = buffer.len() - needle_size_bytes; // - needle_size_bytes: avoid buffer out of bouds read
        let operations: Vec<(String, f32)> = vec![(">".to_string(), 10.0 as f32), ("<".to_string(), 20.0 as f32)];

        // From this point on, the buffer cannot be changed
        let arc_buffer = Arc::new(buffer);

        let result = LinearSearch_Comparator_f32(arc_buffer, start, end, operations, 1000);

        println!("Result: {:?}", result);

        // Did it find the correct start position?
        let expected: Vec<usize> = vec![25];
        assert_eq!(result, expected);
    }
}