use crate::ThreadPool;
use crate::GenericOSInterface;
use crate::SearchEngines;
use crate::Matches;
use std::sync::Arc;
use std::mem;

// I am separating the routines from the Glue code for 2 reasons:
// - It makes the Glue code more high level and easier to follow each step
// - It allows me to test the multithreaded search independently (so I can experiment with different implementations in the future)

// T: Target type
pub fn StartParallelSearchLinearComparator<T: Send + 'static + Clone>(
    arc_copy_buffer: &Arc<Vec<u8>>,
    thread_workload: &mut Vec< Vec<(usize, usize)> >,
    operations: &Vec<(SearchEngines::ComparisonOperation, T)>,
    memory_regions: &[GenericOSInterface::GenericMemoryRegion],
    thread_private_store_size: usize,
    thread_pool: &mut ThreadPool::ThreadPool<
        (
            Vec<(usize, usize)>,
            Arc<Vec<u8>>,
            usize,
            Vec<(SearchEngines::ComparisonOperation, T)>,
            fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, T)], usize) -> Vec<usize>,
            Vec<GenericOSInterface::GenericMemoryRegion>
        ),
        Vec<Vec<usize>> >,
    thread_task: fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, T)], usize) -> Vec<usize>
) -> Vec<Vec<Vec<usize>>>
{
    // Perform the search in parallel

    // Send the task to each thread
    for t_idx in 0..thread_pool.get_num_threads()
    {
        // Each vector is directly associated to a region
        thread_pool.execute(t_idx,
            (mem::take(&mut thread_workload[t_idx]),
            arc_copy_buffer.clone(),
            thread_private_store_size,
            operations.clone(),
            thread_task,
            memory_regions.to_vec()), // aka clone the slice
            |args| -> Vec< Vec<usize> >
        {
            // Unpack args
            let workload = args.0;
            let arc_buffer = args.1;
            let result_buffer_size = args.2;
            let operations = args.3;
            let t_task = args.4;
            let regions = args.5;

            let mut thread_results = Vec::with_capacity(1000);

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

                // Check for mismatched workloads and pages
                // TODO? Ajust for different number of threads
                /*let slice_size = (buff_end-buff_start);
                if slice_size != regions[region_idx].size_bytes
                {
                    panic!("Thread workload slice doesn't match with the page");
                }
                */

                // DEBUG ONLY: $env:RUSTFLAGS='--cfg debug_print="StartParallelSearchLinearComparator"'
                #[cfg(debug_print = "StartParallelSearchLinearComparator")]
                {
                    println!("Current region: {:?}", regions[region_idx]);
                    println!("Current buffer total size: {:?}", arc_buffer.len());
                    println!("Current task: {:?}", region_workload);
                    println!("Start and end: {:?}", (start, end));
                    println!("Start and end buffer: {:?}", (buff_start, buff_end));
                    println!("Start and end buffer slice size: {}", buff_end-buff_start);
                    println!("Start match value: {}", start);
                    println!("\n\n");
                }

                thread_results.push(t_task(
                    &arc_buffer[buff_start..buff_end], // The thread can only read its private segment
                    start, // It is ajust the results to be relative to the buffer
                    &operations,
                    result_buffer_size));

                // Use the buffer size as an offset
                current_buffer_pos = current_buffer_pos + regions[region_idx].size_bytes;

                //println!("T Task: {}/{}", region_idx+1, workload.len());
            }

            return thread_results;
        }).unwrap(); // It panics if we send tasks without first collecting the results
    }

    // Return the search results to be collected
    return thread_pool.wait_all().unwrap();
}

pub fn FilterParallelSearchLinearComparator<T: Send + 'static + Clone>(
    arc_previous_results: &Arc<Vec<Matches::AddressMatches>>,
    start_copy_region: usize,
    copies_done: usize,
    arc_copy_buffer: &Arc<Vec<u8>>,
    thread_workload: &mut Vec< Vec<(usize, usize)> >,
    operations: &Vec<(SearchEngines::ComparisonOperation, T)>,
    memory_regions: &[GenericOSInterface::GenericMemoryRegion],
    thread_private_store_size: usize,
    thread_pool: &mut ThreadPool::ThreadPool<
        (
            Vec<(usize, usize)>,
            Arc<Vec<u8>>,
            usize,
            Vec<(SearchEngines::ComparisonOperation, T)>,
            fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, T)], usize, &[usize]) -> Vec<usize>,
            Vec<GenericOSInterface::GenericMemoryRegion>,
            Arc<Vec<Matches::AddressMatches>>, 
            usize,
            usize
        ),
        Vec<Vec<usize>> >,
    thread_task: fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, T)], usize, &[usize]) -> Vec<usize>
) -> Vec<Vec<Vec<usize>>>
{
        // Send the task to each thread
        for t_idx in 0..thread_pool.get_num_threads()
        {
            // Each vector is directly associated to a region
            thread_pool.execute(t_idx,
                (mem::take(&mut thread_workload[t_idx]),
                arc_copy_buffer.clone(),
                thread_private_store_size,
                operations.clone(),
                thread_task,
                memory_regions.to_vec(), // aka clone the slice
                arc_previous_results.clone(),
                start_copy_region,
                copies_done),
                |args| -> Vec< Vec<usize> >
            {
                // Unpack args
                let workload = args.0;
                let arc_buffer = args.1;
                let result_buffer_size = args.2;
                let operations = args.3;
                let t_task = args.4;
                let regions = args.5;
                let start_copy_region = args.7;
                let copies_done = args.8;
                let previous_matches = &args.6[start_copy_region..(start_copy_region+copies_done)]; // Get only the matches from the copied regions

                let mut thread_results = vec![];

                // DEBUG ONLY
                //println!("Workload:\n{:?}", workload);

                // Check the start search on this variable
                let mut current_buffer_pos: usize = 0;

                // Loop over every region
                for (region_idx, region_workload) in workload.iter().enumerate()
                {
                    let start = region_workload.0;
                    let buff_start = current_buffer_pos;
                    //let buff_start = start + current_buffer_pos;

                    let end = region_workload.1;
                    let buff_end = regions[region_idx].size_bytes + buff_start;
                    //let buff_end = end + current_buffer_pos;

                    // DEBUG ONLY: $env:RUSTFLAGS='--cfg debug_print="FilterParallelSearchLinearComparator"'
                    #[cfg(debug_print = "FilterParallelSearchLinearComparator")]
                    {
                        //println!(" Buffer:\n{:?} \n Matches:\n{:?} \n Slice:\n{:?}", &arc_buffer, &previous_matches[region_idx].matches[start..end], &arc_buffer[current_buffer_pos..(current_buffer_pos+regions[region_idx].size_bytes)]);
                        println!("Current region: {:?}", regions[region_idx]);
                        println!("Current buffer total size: {:?}", arc_buffer.len());
                        println!("Current task: {:?}", region_workload);
                        println!("Start and end: {:?}", (start, end));
                        println!("Start and end buffer: {:?}", (buff_start, buff_end));
                        println!("Start match value: {}", previous_matches[region_idx].matches[0]);
                        println!("Matches: {:?}", previous_matches[region_idx].matches);
                        println!("\n\n");
                    }

                    thread_results.push(t_task(
                        &arc_buffer[buff_start..buff_end], // Filter operations have access to the whole buffer, relative to that region

                        // Since we ajust the pages, we need to also ajust the match value to the new memory (otherwise we can an access out of bounds)
                        // It is also important to mention that we need to use the GLOBAL first match, since the page ajustment only takes in consideration the first and last global matches address (and don't ajust the matches itself)
                        // Failling to do so will cause threads to ajust to their private workloads and consequently misalign the addresses of the search
                        // If you ignore this, you are considering that each thread uses a slice that corresponds to its search matches and that the first position also corresponds to the first match position
                        // However, the assumption here is that the filter operation can access the whole section and only select what it wants from the matches
                        // One symptom is that the filtering removes valid addresses based on the number of threads (even if the process is stopped): T2 -> m/2; T4 -> m/4; T8 -> m/8
                        previous_matches[region_idx].matches[0],
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

        // Give the results to be later collected in order
        return thread_pool.wait_all().unwrap();
}

#[cfg(test)]
mod tests
{
    use crate::SearchRoutines::*;
    use crate::SearchGlue::*;
    use crate::GenericOSInterface::*;
    use crate::WorkloadPartitioning::*;
    use crate::Matches::*;
    use crate::ThreadPool;
    use crate::SearchEngines::*;
    use std::mem::size_of;
    use std::sync::Arc;
    use std::time;

    // Use this test to benchmark search engines
    #[ignore]
    #[test]
    fn TestStartPrallelSearchRoutineBench()
    {
        let num_threads: usize = 16;
        let buffer_size: usize = 32*1024*1024*1024;
        let thread_private_store_size: usize = 100000;

        // Create what would be the representation of the memory in the process
        let mut buffer = vec![0; buffer_size];
        buffer[buffer_size-1] = 1;
        let arc_buffer: Arc<Vec<u8>> = Arc::new(buffer);

        // Create the memory regions, which need to correspond to the copy buffer
        let memory_regions = vec![GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 0, buffer_size)];

        // Calculate the workload for each thread
        let mut thread_workload = partition_thread_workload_equal_slice_view(num_threads, &memory_regions, size_of::<u32>());

        let mut thread_pool = ThreadPool::ThreadPool::<
            (
                Vec<(usize, usize)>,
                Arc<Vec<u8>>,
                usize,
                Vec<(SearchEngines::ComparisonOperation, u32)>,
                fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, u32)], usize) -> Vec<usize>,
                Vec<GenericOSInterface::GenericMemoryRegion>
            ),
            Vec<Vec<usize>> >::new(num_threads).unwrap();

        let operations: Vec<(SearchEngines::ComparisonOperation, u32)> = vec![(ComparisonOperation::Equal, 1)];

        let timer = time::Instant::now();
        let all_results = StartParallelSearchLinearComparator::<u32>(
            &arc_buffer,
            &mut thread_workload,
            &operations,
            &memory_regions,
            thread_private_store_size,
            &mut thread_pool,
            LinearSearch_Comparator_u32
        );
        let elapsed = timer.elapsed();

        println!("Search took: \n{}s\n{}ms\n{}us", elapsed.as_secs(), elapsed.as_millis(), elapsed.as_micros());
        println!("Throughput: \n{} bytes/ms \n{} GiB/s", buffer_size as u128/elapsed.as_millis(), (buffer_size/(1024*1024*1024)) as f64 /elapsed.as_secs() as f64);
        //println!("Results from search: \n{:?}", all_results);

        let expected: Vec<Vec<Vec<usize>>> = vec![ vec![ vec![] ] ];
        //assert_eq!(expected, all_results);
        assert!(false);
    }


    #[test]
    fn TestStartPrallelSearchRoutine_RegularCase()
    {
        let num_threads: usize = 4;
        let buffer_size: usize = 1000;
        let thread_private_store_size: usize = 1000;

        // Create what would be the representation of the memory in the process
        let mut buffer = vec![0; buffer_size];
        buffer[500] = 1;
        let arc_buffer: Arc<Vec<u8>> = Arc::new(buffer);

        // Create the memory regions, which need to correspond to the copy buffer
        let memory_regions = vec![GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 0, buffer_size)];

        // Calculate the workload for each thread
        let mut thread_workload = partition_thread_workload_equal_slice_view(num_threads, &memory_regions, size_of::<u8>());

        println!("Workload: {:?}", thread_workload);

        let mut thread_pool = ThreadPool::ThreadPool::<
            (
                Vec<(usize, usize)>,
                Arc<Vec<u8>>,
                usize,
                Vec<(SearchEngines::ComparisonOperation, u8)>,
                fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, u8)], usize) -> Vec<usize>,
                Vec<GenericOSInterface::GenericMemoryRegion>
            ),
            Vec<Vec<usize>> >::new(num_threads).unwrap();

        let operations: Vec<(SearchEngines::ComparisonOperation, u8)> = vec![(ComparisonOperation::Equal, 1)];

        let timer = time::Instant::now();
        let all_results = StartParallelSearchLinearComparator(
            &arc_buffer,
            &mut thread_workload,
            &operations,
            &memory_regions,
            thread_private_store_size,
            &mut thread_pool,
            LinearSearch_Comparator_u8
        );
        let elapsed = timer.elapsed();

        println!("Search took: \n{}s\n{}ms\n{}us", elapsed.as_secs(), elapsed.as_millis(), elapsed.as_micros());
        println!("Throughput: \n{} bytes/ms \n{} GiB/s", buffer_size as f64/elapsed.as_millis() as f64, (buffer_size/(1024*1024*1024)) as f64 /elapsed.as_secs() as f64);
        println!("Results from search: \n{:?}", all_results);

        let expected: Vec<Vec<Vec<usize>>> = vec![ vec![vec![]], vec![vec![]], vec![vec![500]], vec![vec![]] ];
        assert_eq!(expected, all_results);
    }


    #[test]
    fn TestStartPrallelSearchRoutine_ValueInTheMiddleOfSegment()
    {
        let num_threads: usize = 4;
        let buffer_size: usize = 1000;
        let thread_private_store_size: usize = 1000;

        // Create what would be the representation of the memory in the process
        let mut buffer = vec![0; buffer_size];
        buffer[499] = 1;
        let arc_buffer: Arc<Vec<u8>> = Arc::new(buffer);

        // Create the memory regions, which need to correspond to the copy buffer
        let memory_regions = vec![GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 0, buffer_size)];

        // Calculate the workload for each thread
        let mut thread_workload = partition_thread_workload_equal_slice_view(num_threads, &memory_regions, size_of::<u16>());

        println!("Workload: {:?}", thread_workload);

        let mut thread_pool = ThreadPool::ThreadPool::<
            (
                Vec<(usize, usize)>,
                Arc<Vec<u8>>,
                usize,
                Vec<(SearchEngines::ComparisonOperation, u16)>,
                fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, u16)], usize) -> Vec<usize>,
                Vec<GenericOSInterface::GenericMemoryRegion>
            ),
            Vec<Vec<usize>> >::new(num_threads).unwrap();

        let operations: Vec<(SearchEngines::ComparisonOperation, u16)> = vec![(ComparisonOperation::Equal, 1)];

        let timer = time::Instant::now();
        let all_results = StartParallelSearchLinearComparator(
            &arc_buffer,
            &mut thread_workload,
            &operations,
            &memory_regions,
            thread_private_store_size,
            &mut thread_pool,
            LinearSearch_Comparator_u16
        );
        let elapsed = timer.elapsed();

        println!("Search took: \n{}s\n{}ms\n{}us", elapsed.as_secs(), elapsed.as_millis(), elapsed.as_micros());
        println!("Throughput: \n{} bytes/ms \n{} GiB/s", buffer_size as f64/elapsed.as_millis() as f64, (buffer_size/(1024*1024*1024)) as f64 /elapsed.as_secs() as f64);
        println!("Results from search: \n{:?}", all_results);

        let expected: Vec<Vec<Vec<usize>>> = vec![ vec![vec![]], vec![vec![499]], vec![vec![]], vec![vec![]] ];
        assert_eq!(expected, all_results);
    }

    #[test]
    fn TestStartPrallelFilterSearchRoutine_RegularCase_MultipleRegions()
    {
        let num_threads: usize = 4;
        let buffer_size: usize = 1000;
        let num_regions: usize = 4;
        let thread_private_store_size: usize = 1000;
        
        // Create what would be the representation of the memory in the process
        let mut buffer = vec![0; buffer_size*num_regions];
        buffer[36] = 1;
        println!("Needle look: {:?}", u32::to_ne_bytes(1));
        let arc_buffer: Arc<Vec<u8>> = Arc::new(buffer);
        
        // Create the memory regions, which need to correspond to the copy buffer
        let memory_regions = vec![
            GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 0, buffer_size),
            GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 1000, buffer_size),
            GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 2000, buffer_size),
            GenericMemoryRegion::new(PageProtection_Read|PageProtection_Write, GenericRegionState::Resident, 3000, buffer_size)
            ];

        let previous_matches: Arc<Vec<AddressMatches>> = Arc::new(vec![
            AddressMatches::new(memory_regions[0].clone(), vec![960, 996]), // 0 - 40
            AddressMatches::new(memory_regions[1].clone(), vec![800]), // 40 - 44
            AddressMatches::new(memory_regions[2].clone(), vec![500]), // 44 - 48
            AddressMatches::new(memory_regions[2].clone(), vec![700]) // 48 - 52
            ]);

        let ajusted_pages = ajust_pages_min_max(&previous_matches, size_of::<u32>()).unwrap();

            // Calculate the workload for each thread
        let mut thread_workload = partition_thread_workload_equal_slice_view_filter(num_threads, &previous_matches);

        println!("Workload: {:?}", thread_workload);
        println!("Original regions \n-------\n{:#?}", memory_regions);
        println!("Ajusted regions \n-------\n{:#?}", ajusted_pages);

        let mut thread_pool = ThreadPool::ThreadPool::<
            (Vec<(usize, usize)>,
            Arc<Vec<u8>>,
            usize,
            Vec<(SearchEngines::ComparisonOperation, u32)>,
            fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, u32)], usize, &[usize]) -> Vec<usize>,
            Vec<GenericOSInterface::GenericMemoryRegion>,
            Arc<Vec<Matches::AddressMatches>>,
            usize,
            usize),
            Vec<Vec<usize>>
        >::new(num_threads).unwrap();

        let operations: Vec<(SearchEngines::ComparisonOperation, u32)> = vec![(ComparisonOperation::Equal, 1)];

        let timer = time::Instant::now();

        let all_results = FilterParallelSearchLinearComparator(
            &previous_matches,
            0,
            4,
            &arc_buffer,
            &mut thread_workload,
            &operations,
            &ajusted_pages,
            thread_private_store_size,
            &mut thread_pool,
            LinearSearch_ComparatorFilter_u32
        );

        let elapsed = timer.elapsed();

        println!("Search took: \n{}s\n{}ms\n{}us", elapsed.as_secs(), elapsed.as_millis(), elapsed.as_micros());
        println!("Throughput: \n{} bytes/ms \n{} GiB/s", buffer_size as f64/elapsed.as_millis() as f64, (buffer_size/(1024*1024*1024)) as f64 /elapsed.as_secs() as f64);
        println!("Results from search: \n{:?}", all_results);

        let expected: Vec<Vec<Vec<usize>>> = vec![

            // Thread 0
            vec![
                vec![], // Region 0
                vec![], // Region 1
                vec![], // Region 2
                vec![] // Region 3
            ],

            // Thread 1
            vec![
                vec![996], // Region 0
                vec![], // Region 1
                vec![], // Region 2
                vec![] // Region 3
            ],

            // Thread 2
            vec![
                vec![], // Region 0
                vec![], // Region 1
                vec![], // Region 2
                vec![] // Region 3
            ],

            // Thread 3
            vec![
                vec![], // Region 0
                vec![], // Region 1
                vec![], // Region 2
                vec![] // Region 3
            ],
        ];
    
        assert_eq!(expected, all_results);
    }
}