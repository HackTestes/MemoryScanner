use crate::Matches;
use crate::ThreadPool;
use crate::GenericOSInterface;
use crate::SearchEngines;
use crate::WorkloadPartitioning;
use crate::ResultMergerHelpers;
use std::sync::Arc;
use std::mem::size_of;
use std::mem;

#[derive(Debug)]
#[derive(PartialEq)]
enum SearchErrors
{
    TargetTypeTooBig,
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
    thread_task: fn(&[u8], usize, &[(String, T)], usize) -> Vec<usize>,
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
                                                    fn(&[u8], usize, &[(String, T)], usize) -> Vec<usize>,
                                                    Vec<GenericOSInterface::GenericMemoryRegion>),
                                                    Vec<Vec<usize>> >::new(num_threads);

    // Loops over the the copy operations needed
    for start_copy_position in snapshot_workload
    {
        // Create a snapshot of the process (copy it to the buffer)
        let snapshot_result = process_handle.snapshot_bounded(&memory_regions[(start_copy_position)..], &mut copy_buffer[0..]);

        // Check for errors
        let copies_done = match snapshot_result
        {
            Ok(num_copies) => num_copies,

            // If there is any error, return immediately
            Err(error) => return Err(SearchErrors::OSInterfaceError(error)),
        };

        // Create the workload partitioning for that particular buffer, you must consider the target type for the search
        let mut thread_workload = WorkloadPartitioning::partition_thread_workload_equal_slice_view(num_threads, &memory_regions[start_copy_position..(start_copy_position+copies_done)], size_of::<T>());

        // Perform the search in parallel

        // Make the buffer shareable
        let arc_copy_buffer = Arc::new(copy_buffer);

        // Send the task to each thread
        for t_idx in 0..num_threads
        {
            // Each vector is directly associated to a region
            thread_pool.execute(t_idx,
                (mem::take(&mut thread_workload[t_idx]),
                arc_copy_buffer.clone(),
                thread_private_store_size,
                operations.clone(),
                thread_task,
                memory_regions.clone()),
                |args| -> Vec< Vec<usize> >
            {
                // Unpack args
                let workload = args.0;
                let arc_buffer = args.1;
                let result_buffer_size = args.2;
                let operations = args.3;
                let t_task = args.4;
                let regions = args.5;

                let mut thread_results = vec![];

                // This value is used so we can get the correct region from the buffer
                // Aka I am getting the region's position in the buffer
                // We start at zero and then increase based on the size
                let mut current_buffer_pos: usize = 0;

                // Loop over every region
                for (region_idx, region_workload) in workload.iter().enumerate()
                {
                    let start = region_workload.0;
                    let buff_start = start + current_buffer_pos;

                    let end = region_workload.1;
                    let buff_end = end + current_buffer_pos;

                    // Use it only for debugging searched regions
                    //println!("Region relative:({}, {})", start, end);
                    //println!("Buffer: ({}, {})", buff_start, buff_end);
                    //println!("{:?}", arc_buffer);

                    thread_results.push(t_task(
                        &arc_buffer[buff_start..buff_end], // The thread can only read its private segment
                        start, // It is ajust the results to be relative to the buffer
                        &operations,
                        result_buffer_size));

                    // Use the buffer size as an offset
                    current_buffer_pos = current_buffer_pos + regions[region_idx].size_bytes;
                }

                return thread_results;
            }).unwrap(); // It panics if we send tasks without first collecting the results
        }

        // Collect the results for that buffer (in order)
        let all_results = thread_pool.wait_all().unwrap();

        // Give the buffer back its ownership
        copy_buffer = Arc::try_unwrap(arc_copy_buffer).unwrap();

        // Now merge everything in order for each of the pages copied in the buffer
        ResultMergerHelpers::MergeLinearSearchResults(
            all_results,
            &memory_regions[start_copy_position..(start_copy_position+copies_done)],
            num_threads,
            &mut search_results
        );
        
        /*
        for (region_idx, region) in memory_regions[start_copy_position..(start_copy_position+copies_done)].iter().enumerate()
        {
            let mut region_matches: Vec<usize> = vec![];

            // Gettings the results in the same order as threads also returns ordered results
            // This is why each result index is associated to a given region
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
        */
    }

    return Ok(search_results);
}

// Filter helper functions

// Ajust pages
// The idea is to modify page information to account for the min, max and target type, so we can reuse most of the code from other places
// This is also important, because it allows me to fit more regions into a buffer and use less memory (it also reduces the amount of synchronization calls)
fn ajust_pages_min_max(previous_matches: &[Matches::AddressMatches], target_type_size_bytes: usize) -> Result<Vec<GenericOSInterface::GenericMemoryRegion>, SearchErrors>
{
    let mut ajusted_mem_regions: Vec<GenericOSInterface::GenericMemoryRegion> = vec![];

    for region_match in previous_matches
    {
        // Get min and max addresses
        // I am assuming that the result is ALWAYS ordered, if this assumption is incorrect, it will cause a panic (bounds checking)
        // Why not use a Max and Min function? It has an O(n) complexity, and might take a significant performance hit
        let min = region_match.matches[0];
        let max = region_match.matches.last().unwrap();

        // Check is max address is valid for the target type
        // We need to take into account that the max address is also read from the memory, so we subtract 1
        if max + target_type_size_bytes -1 >= region_match.mem_region.size_bytes
        {
            // It extrapolates the region size
            // Alert the caller that the type size is not good, and can will lose some results
            // All u32 results are valid u8, but not all u8 are valid u32
            // I prefer to simply refuse the search as it also simplifies the code
            // Another consideration is that we would need to remove all offending matches in all regions,
            // something that would have a O(s*p) complexity - s being the type size and p the amount of pages
            return Err(SearchErrors::TargetTypeTooBig);
        }

        // Create a copy of the memory region and ajust it
        let mut mem_region_copy = region_match.mem_region.clone();

        // We move the base all the way to the min value, since matches only look foward
        // +mem_region_copy.base_address : otherwise, it clears the base address
        mem_region_copy.base_address = mem_region_copy.base_address + min;

        // The max also needs to take into account the target size in bytes
        // Remember that the end is NON INCLUSIVE
        mem_region_copy.size_bytes = (max - min) + target_type_size_bytes;

        ajusted_mem_regions.push(mem_region_copy);
    }

    return Ok(ajusted_mem_regions);
}

fn FilterSearchComparator<T: Send + 'static + Clone>(
    previous_results: Vec<Matches::AddressMatches>,
    process_handle: GenericOSInterface::GenericProcess,
    num_threads: usize,
    buffer_size: usize,
    thread_private_store_size: usize,
    thread_task: fn(&[u8], usize, &[(String, T)], usize, &[usize]) -> Vec<usize>,
    operations: Vec<(String, T)>) -> Result<Vec<Matches::AddressMatches>, SearchErrors>
{
    let mut search_results: Vec<Matches::AddressMatches> = Vec::with_capacity(10240);

    // Get all the pages from the previous results
    // Store a copy of all of the regions that will search
    //let memory_regions: Vec<GenericOSInterface::GenericMemoryRegion> = previous_results.iter().map(|x| x.mem_region.clone()).collect();
    let memory_regions_r = ajust_pages_min_max(&previous_results, size_of::<T>());

    let memory_regions: Vec<GenericOSInterface::GenericMemoryRegion> = match memory_regions_r
    {
        Ok(ajusted_pages) => ajusted_pages,
        Err(error) => return Err(error),
    };

    let original_memory_regions: Vec<GenericOSInterface::GenericMemoryRegion> = previous_results.iter().map(|x| x.mem_region.clone()).collect();

    // Make the previous results sharable
    let arc_previous_results = Arc::new(previous_results);

    // Allocate a buffer to store the copies of regions of the target, the size is controlled by the caller
    let mut copy_buffer: Vec<u8> = vec![0; buffer_size];

    // Calculate if the search is possible with such buffer
    // A new way to calculte how much memory is necessary should take into account the min and max addresses
    // The max value has another problem, it also needs to consider the size of the target
    // But this was done by ajusting the pages
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
                                                    fn(&[u8], usize, &[(String, T)], usize, &[usize]) -> Vec<usize>,
                                                    Vec<GenericOSInterface::GenericMemoryRegion>,
                                                    Arc<Vec<Matches::AddressMatches>>),
                                                    Vec<Vec<usize>> >::new(num_threads);

    // Loops over the the copy operations needed
    for start_copy_position in snapshot_workload
    {
        // Create a snapshot of the process (copy it to the buffer)
        let snapshot_result = process_handle.snapshot_bounded(&memory_regions[start_copy_position..], &mut copy_buffer[0..]);

        // Check for errors
        let copies_done = match snapshot_result
        {
            Ok(num_copies) => num_copies,

            // If there is any error, return immediately
            Err(error) => return Err(SearchErrors::OSInterfaceError(error)),
        };

        // Create the workload partitioning for that particular buffer, you must consider the target type for the search
        let mut thread_workload = WorkloadPartitioning::partition_thread_workload_equal_slice_view_filter(num_threads, &arc_previous_results[start_copy_position..(start_copy_position+copies_done)]);

        // Perform the search in parallel

        // Make the buffer shareable
        let arc_copy_buffer = Arc::new(copy_buffer);

        // Send the task to each thread
        for t_idx in 0..num_threads
        {
            // Each vector is directly associated to a region
            thread_pool.execute(t_idx,
                (mem::take(&mut thread_workload[t_idx]),
                arc_copy_buffer.clone(),
                thread_private_store_size,
                operations.clone(),
                thread_task,
                memory_regions.clone(),
                arc_previous_results.clone()),
                |args| -> Vec< Vec<usize> >
            {
                // Unpack args
                let workload = args.0;
                let arc_buffer = args.1;
                let result_buffer_size = args.2;
                let operations = args.3;
                let t_task = args.4;
                let regions = args.5;
                let previous_matches = args.6;

                let mut thread_results = vec![];

                // DEBUG ONLY
                //println!("Workload:\n{:?}", workload);

                // Check the start search on this variable
                let mut current_buffer_pos: usize = 0;

                // Loop over every region
                for (region_idx, region_workload) in workload.iter().enumerate()
                {
                    let start = region_workload.0;
                    let buff_start = start + current_buffer_pos;

                    let end = region_workload.1;
                    let buff_end = end + current_buffer_pos;

                    // DEBUG ONLY
                    //println!(" Buffer:\n{:?} \n Matches:\n{:?} \n Slice:\n{:?}", &arc_buffer, &previous_matches[region_idx].matches[start..end], &arc_buffer[current_buffer_pos..(current_buffer_pos+regions[region_idx].size_bytes)]);

                    thread_results.push(t_task(
                        &arc_buffer[buff_start..buff_end], // Filter operations have access to the whole buffer, relative to that region
                        previous_matches[region_idx].matches[start], // Since we ajust the pages, we need to also ajust the match value to the new memory (otherwise we can an access out of bounds)
                        &operations,
                        result_buffer_size,
                        &previous_matches[region_idx].matches[start..end])); // We now limit which matches the thread can read for each region

                    // Use the buffer size as an offset
                    current_buffer_pos = current_buffer_pos + regions[region_idx].size_bytes;
                }

                // DEBUG ONLY
                //println!("Thread r:\n{:?}", &thread_results);

                // Remember that the results are ajusted back, becoming relative to the original regions
                return thread_results;
            });
        }

        // Collect the results for that buffer (in order)
        let all_results = thread_pool.wait_all().unwrap();

        // Give the buffer back its ownership
        copy_buffer = Arc::try_unwrap(arc_copy_buffer).unwrap();

        // Now merge everything in order for each of the pages copied in the buffer
        ResultMergerHelpers::MergeLinearSearchResults(
            all_results,
            &original_memory_regions[start_copy_position..(start_copy_position+copies_done)],
            num_threads,
            &mut search_results
        );
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
    fn TestStartSearchRegularCase()
    {
        let process = GenericProcess::attach(1).unwrap();

        let search_result = StartSearchComparator(
            PageProtection_Read|PageProtection_Write,
            None,
            None,
            process,
            8,
            100,
            1000,
            LinearSearch_Comparator_u32, // It is possible to infer the type from this function
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

    // This test helps me to verify if the search is reading the different regions, so we try to find an unique pattern
    #[test]
    fn TestStartSearchRegularCaseByteLong()
    {
        let process = GenericProcess::attach(1).unwrap();

        let search_result = StartSearchComparator(
            PageProtection_Read|PageProtection_Write,
            None,
            None,
            process,
            8,
            500,
            1000,
            LinearSearch_Comparator_u8, // It is possible to infer the type from this function
            vec![(">=".to_string(), 99)]
        ).unwrap();

        for region_match in search_result.iter()
        {
            println!("Search: {}", region_match.display_matches(MatchDisplayStyle::Decimal));
        }

        // This checks not only if the pages are correct, but also that the pages came in order
        // This is important for the result filter, so min and max operations can be fast
        let expected: Vec<AddressMatches> = vec![
            // The matches represent the relative address in the region, not the value itself (so count the matches backwords)
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100), (94..100).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), (93..100).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 900, 100), (90..100).collect()),
            ];
        assert_eq!(search_result, expected);
    }

    #[test]
    // It needs to reuse the buffer multiple times
    fn TestStartSearchSmallBuffer()
    {
        let process = GenericProcess::attach(1).unwrap();

        let search_result = StartSearchComparator(
            PageProtection_Read|PageProtection_Write,
            None,
            None,
            process,
            8,
            100,
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

    #[test]
    fn TestStartSearchFail()
    {
        let process = GenericProcess::attach(1).unwrap();

        let search_result = StartSearchComparator(
            PageProtection_Read|PageProtection_Write,
            None,
            None,
            process,
            8,
            1,
            1000,
            LinearSearch_Comparator_u32,
            vec![(">".to_string(), 0)]
        );

        let expected = SearchErrors::OSInterfaceError(GenericOSErrors::SnapshotBufferIsTooSmall);
        assert_eq!(search_result, Err(expected));
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

    #[test]
    fn TestFilterRegionAjustment_RegularCase()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            100,
            1000);

        let matches_obj = vec![
            AddressMatches::new(memory_region.clone(), vec![0, 500] )
            ];

        // 4 byte long target
        let ajusted_pages = ajust_pages_min_max(&matches_obj, size_of::<u8>());
        println!("{:?}", ajusted_pages);
        
        let mut expected_region = memory_region.clone();
        expected_region.base_address = 100;
        expected_region.size_bytes = 501;

        assert_eq!(vec![expected_region], ajusted_pages.unwrap());
    }

    #[test]
    fn TestFilterRegionAjustment_RegularCase_PageStaysTheSame()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            500,
            1000);

        let matches_obj = vec![
            AddressMatches::new(memory_region.clone(), vec![0, 999] )
            ];

        // 4 byte long target
        let ajusted_pages = ajust_pages_min_max(&matches_obj, size_of::<u8>());
        println!("{:?}", ajusted_pages);
        
        let mut expected_region = memory_region.clone();
        expected_region.base_address = 500;
        expected_region.size_bytes = 1000;

        assert_eq!(vec![expected_region], ajusted_pages.unwrap());
    }

    #[test]
    fn TestFilterRegionAjustment_RegularCase_MinMaxIsTheSame()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            10,
            1000);

        let matches_obj = vec![
            AddressMatches::new(memory_region.clone(), vec![500] )
            ];

        // 4 byte long target
        let ajusted_pages = ajust_pages_min_max(&matches_obj, size_of::<u32>());
        println!("{:?}", ajusted_pages);
        
        let mut expected_region = memory_region.clone();
        expected_region.base_address = 510;
        expected_region.size_bytes = 4; // Non inclusive

        assert_eq!(vec![expected_region], ajusted_pages.unwrap());
    }

    #[test]
    fn TestFilterRegionAjustment_RegularCase_MultipleRegions()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            0,
            1000);

        let matches_obj = vec![
            AddressMatches::new(memory_region.clone(), vec![5, 100] ),
            AddressMatches::new(memory_region.clone(), vec![15, 16, 17, 467] ),
            AddressMatches::new(memory_region.clone(), vec![155] ),
            AddressMatches::new(memory_region.clone(), vec![500, 667, 705] )
            ];

        // 4 byte long target
        let ajusted_pages = ajust_pages_min_max(&matches_obj, size_of::<u8>());
        println!("{:?}", ajusted_pages);

        let mut expected_regions = vec![
            memory_region.clone(),
            memory_region.clone(),
            memory_region.clone(),
            memory_region.clone()
            ];

        // Expected region ajustment
        expected_regions[0].base_address = 5;
        expected_regions[0].size_bytes = 96;

        expected_regions[1].base_address = 15;
        expected_regions[1].size_bytes = 453;

        expected_regions[2].base_address = 155;
        expected_regions[2].size_bytes = 1;

        expected_regions[3].base_address = 500;
        expected_regions[3].size_bytes = 206;

        assert_eq!( expected_regions, ajusted_pages.unwrap());
    }

    #[test]
    fn TestFilterRegionAjustment_TargetBiggerThan1Byte()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            50,
            1000);

        let matches_obj = vec![
            AddressMatches::new(memory_region.clone(), vec![100, 996] )
            ];

        // 4 byte long target
        let ajusted_pages = ajust_pages_min_max(&matches_obj, size_of::<u32>());
        println!("{:?}", ajusted_pages);
        
        let mut expected_region = memory_region.clone();
        expected_region.base_address = 150;
        expected_region.size_bytes = 900; // Non inclusive

        assert_eq!(vec![expected_region], ajusted_pages.unwrap());
    }

    #[test]
    fn TestFilterRegionAjustment_TargetBiggerThan1Byte_Fail()
    {
        let memory_region = GenericOSInterface::GenericMemoryRegion::new(
            GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write,
            GenericOSInterface::GenericRegionState::Resident,
            0,
            1000);

        let matches_obj = vec![
            AddressMatches::new(memory_region.clone(), vec![100, 997] )
            ];

        // 4 byte long target
        let ajusted_pages = ajust_pages_min_max(&matches_obj, size_of::<u32>());
        println!("{:?}", ajusted_pages);

        assert_eq!(Err(SearchErrors::TargetTypeTooBig), ajusted_pages);
    }

    #[test]
    fn TestFilterSearch_RegularCase()
    {
        let process = GenericProcess::attach(1).unwrap();

        let search_result = StartSearchComparator(
            PageProtection_Read|PageProtection_Write,
            None,
            None,
            process.clone(),
            1,
            500,
            1000,
            LinearSearch_Comparator_u8, // It is possible to infer the type from this function
            vec![(">".to_string(), 0)]
        ).unwrap();

        for region_match in search_result.iter()
        {
            println!("Search: {}", region_match.display_matches(MatchDisplayStyle::Decimal));
        }

        // This checks not only if the pages are correct, but also that the pages came in order
        // This is important for the result filter, so min and max operations can be fast
        let expected: Vec<AddressMatches> = vec![
            // The matches represent the relative address in the region, not the value itself (so count the matches backwords)
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100), (0..100).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), (0..100).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 900, 100), (0..100).collect()),
            ];
        assert_eq!(search_result, expected);

        let filter_result = FilterSearchComparator(
            search_result,
            process.clone(),
            2,
            500,
            1000,
            LinearSearch_ComparatorFilter_u8, // It is possible to infer the type from this function
            vec![("==".to_string(), 10)]
        ).unwrap();

        let expected_filter: Vec<AddressMatches> = vec![
            // The matches represent the relative address in the region, not the value itself (so count the matches backwords)
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100), (5..6).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), (4..5).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 900, 100), (1..2).collect())
            ];
        assert_eq!(filter_result, expected_filter);
    }

    
    // This is to make sure that the results return the original regions
    #[test]
    fn TestFilterSearch_RegularCase_FilterTheFilteredResults()
    {
        let process = GenericProcess::attach(1).unwrap();

        let expected_search_result: Vec<AddressMatches> = vec![
            // The matches represent the relative address in the region, not the value itself (so count the matches backwords)
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100), (50..51).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), (50..51).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 900, 100), (50..51).collect()),
            ];

        let mut filter_result = FilterSearchComparator(
            expected_search_result,
            process.clone(),
            3,
            500,
            1000,
            LinearSearch_ComparatorFilter_u8, // It is possible to infer the type from this function
            vec![("==".to_string(), 56)]
        ).unwrap();

        let expected_filter: Vec<AddressMatches> = vec![
            // The matches represent the relative address in the region, not the value itself (so count the matches backwords)
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), (50..51).collect()),
            ];
        assert_eq!(filter_result, expected_filter);

        filter_result = FilterSearchComparator(
            filter_result,
            process.clone(),
            1,
            100,
            1000,
            LinearSearch_ComparatorFilter_u8, // It is possible to infer the type from this function
            vec![("<".to_string(), 10)]
        ).unwrap();

        let expected_filter: Vec<AddressMatches> = vec![];
        assert_eq!(filter_result, expected_filter);
    }

    #[test]
    fn TestFilterSearch_RegularCase_SmallBuffer()
    {
        let process = GenericProcess::attach(1).unwrap();

        let expected_search_result: Vec<AddressMatches> = vec![
            // The matches represent the relative address in the region, not the value itself (so count the matches backwords)
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100), (50..51).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), (50..51).collect()),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 900, 100), (50..51).collect()),
            ];

        let mut filter_result = FilterSearchComparator(
            expected_search_result,
            process.clone(),
            3,
            100,
            1000,
            LinearSearch_ComparatorFilter_u8, // It is possible to infer the type from this function
            vec![("==".to_string(), 56)]
        ).unwrap();

        let expected_filter: Vec<AddressMatches> = vec![
            // The matches represent the relative address in the region, not the value itself (so count the matches backwords)
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), (50..51).collect()),
            ];
        assert_eq!(filter_result, expected_filter);
    }

    #[test]
    fn TestFilterSearch_RegularCase_Fail_TypeTooBig()
    {
        let process = GenericProcess::attach(1).unwrap();

        let expected_search_result: Vec<AddressMatches> = vec![
            // The matches represent the relative address in the region, not the value itself (so count the matches backwords)
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write|PageProtection_Execute, GenericRegionState::Resident, 500, 100), vec![99]),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 600, 100), vec![99]),
            AddressMatches::new(GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 900, 100), vec![99]),
            ];

        let mut filter_result = FilterSearchComparator(
            expected_search_result,
            process.clone(),
            1,
            100,
            1000,
            LinearSearch_ComparatorFilter_u64, // It is possible to infer the type from this function
            vec![("==".to_string(), 56)]
        );

        assert_eq!(filter_result, Err(SearchErrors::TargetTypeTooBig));
    }
}