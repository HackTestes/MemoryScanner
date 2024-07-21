
#[cfg(test)]
mod tests
{
    // Measure the time it takes for to get the min/max value
    // One assumes the vector is unordered and reads all items
    // The other assumes the vector is ordered and only takes the las item
    #[ignore]
    #[test]
    fn TestMatches_Time()
    {
        use std::time;
        use std::cmp;

        let mut vector: Vec<u8> = vec![0; 1000000000];
        let vec_last = vector.len()-1;
        vector[ vec_last ] = 2;

        let mut now = time::Instant::now();

        let min = vector.iter().max().unwrap();

        let time_elapsed = now.elapsed();

        println!("Min: {}", min);
        println!("Time elapsed: {}ms", time_elapsed.as_millis());
        println!("Time elapsed: {}us", time_elapsed.as_micros());

        // Fast unsafe
        let mut now = time::Instant::now();

        let min = vector[vec_last];

        let time_elapsed = now.elapsed();

        println!("Min: {}", min);
        println!("Time elapsed: {}ms", time_elapsed.as_millis());
        println!("Time elapsed: {}us", time_elapsed.as_micros());

        assert!(false);
    }
}