use crate::Matches;
use crate::ThreadPool;
use crate::GenericOSInterface;
use crate::SearchEngines;
use crate::WorkloadPartitioning;
use std::sync::Arc;
use std::mem::size_of;
use std::mem;

#[derive(Debug)]
enum SearchErrors
{
    something,
    OSInterfaceError(GenericOSInterface::GenericOSErrors)
}

enum TargetType
{
//    hex_pattern,
//    string,
    f32,
    f64,
    i8,
    i16,
    i32,
    i64,
    i128,
    u8,
    u16,
    u32,
    u64,
    u128
}


fn StartSearchComparator<T: Send + 'static + Clone>(
    page_permissions_at_least: GenericOSInterface::GenericPageProtections,
    page_permissions_exact: Option<GenericOSInterface::GenericPageProtections>,
    region_state: Option<GenericOSInterface::GenericRegionState>,
    process_handle: GenericOSInterface::GenericProcess,
    num_threads: usize,
    buffer_size: usize,
    thread_private_store_size: usize,
    thread_task: fn(&[u8], usize, Vec<(String, T)>, usize) -> Vec<usize>,
    operations: Vec<(String, T)>) -> Result<Vec<Matches::AddressMatches>, SearchErrors>
{
    let mut search_results: Vec<Matches::AddressMatches> = Vec::with_capacity(10240);

    // Get all pages
    let memory_regions = process_handle.get_mem_regions_info(page_permissions_at_least, page_permissions_exact, region_state).unwrap();

    // Allocate a buffer to store the copies of regions of the target, the size is controlled by the caller
    let mut copy_buffer: Vec<u8> = vec![0; buffer_size];
    
    // Calculate if the search is possible with such buffer
    let snapshot_workload_result = GenericOSInterface::GenericProcess::get_snapshot_workload(&memory_regions, copy_buffer.len());

    let snapshot_workload = match snapshot_workload_result
    {
        Ok(work) => work,

        // If there is any error, return immediately
        Err(error) => return Err(SearchErrors::OSInterfaceError(error)),
    };

    // Create the thread pool
    let mut thread_pool = ThreadPool::ThreadPool::< (Vec<(usize, usize)>,
                                                    Arc<Vec<u8>>,
                                                    usize,
                                                    Vec<(String, T)>,
                                                    fn(&[u8], usize, Vec<(String, T)>, usize) -> Vec<usize>),
                                                    Vec<Vec<usize>> >::new(num_threads);

    // Loops over the the copy operations needed
    for start_copy_position in snapshot_workload
    {
        // Create a snapshot of the process (copy it to the buffer)
        let snapshot_result = process_handle.snapshot_bounded(&memory_regions[(start_copy_position)..], &mut copy_buffer[0..]);

        // Check for errors
        match snapshot_result
        {
            Ok(_) => {},

            // If there is any error, return immediately
            Err(error) => return Err(SearchErrors::OSInterfaceError(error)),
        };

        // Create the workload partitioning for that particular buffer, you must consider the target type for the search
        let mut thread_workload = WorkloadPartitioning::partition_thread_workload_equal_slice_view(num_threads, &memory_regions, size_of::<T>());

        // Perform the search in parallel

        // Make the buffer shareable
        let arc_copy_buffer = Arc::new(copy_buffer);

        // Send the task to each thread
        for t_idx in 0..num_threads
        {
            // Each vector is directly associated to a region
            thread_pool.execute(t_idx, (mem::take(&mut thread_workload[t_idx]), arc_copy_buffer.clone(), thread_private_store_size, operations.clone(), thread_task), |args| -> Vec< Vec<usize> >
            {
                // Unpack args
                let workload = args.0;
                let arc_buffer = args.1;
                let result_buffer_size = args.2;
                let operations = args.3;
                let t_task = args.4;

                let mut thread_results = vec![];

                // Loop over every region
                for region_workload in workload
                {
                    let start = region_workload.0;
                    let end = region_workload.1;

                    // TODO - operarations should be borrowed to avoid unecessary allocations
                    thread_results.push(t_task(&arc_buffer[start..end], start, operations.clone(), result_buffer_size));
                }

                return thread_results;
            });
        }

        // Collect the results for that buffer (in order)
        let all_results = thread_pool.wait_all().unwrap();

        // Give the buffer back its ownership
        copy_buffer = Arc::try_unwrap(arc_copy_buffer).unwrap();

        // Now merge everything in order
        for (region_idx, region) in memory_regions.iter().enumerate()
        {
            let mut region_matches: Vec<usize> = vec![];

            // Gettings the results in the same order as threads also returns ordered results
            for t_idx in 0..num_threads
            {
                region_matches.extend( &all_results[t_idx][region_idx] );
            }

            // Store the result relative to all threads and append the region
            // One should only store results that exist
            if region_matches.len() != 0
            {
                search_results.push( Matches::AddressMatches::new(region.clone(), region_matches) );
            }
        }
    }

    return Ok(search_results);
}




#[cfg(test)]
mod tests
{
    use crate::SearchGlue::*;
    use crate::GenericOSInterface::*;
    use crate::SearchEngines::*;
    use crate::Matches::*;

    #[test]
    fn TestStartSearch()
    {
        let process = GenericProcess::attach(1).unwrap();

        let search_result = StartSearchComparator(
            PageProtection_Read|PageProtection_Write,
            None,
            None,
            process,
            8,
            1000,
            1000,
            LinearSearch_Comparator_u32,
            vec![(">".to_string(), 0)]
        ).unwrap();

        for region_match in search_result.iter()
        {
            println!("Search: {}", region_match.display_matches(MatchDisplayStyle::Decimal));
        }

        // This checks not only if the pages are correct, but also that the pages came in order
        // This is important for the result filter, so min and max operations can be fast
        let expected: Vec<AddressMatches> = vec![
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100), (0..=96).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), (0..=96).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 900, 100), (0..=96).collect())
            ];
        assert_eq!(search_result, expected);
    }

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