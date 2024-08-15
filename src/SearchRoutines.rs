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
    memory_regions: &Vec<GenericOSInterface::GenericMemoryRegion>,
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

                // DEBUG ONLY
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

    // Return the search results to be collected
    return thread_pool.wait_all().unwrap();
}

pub fn FilterParallelSearchLinearComparator<T: Send + 'static + Clone>(
    arc_previous_results: &Arc<Vec<Matches::AddressMatches>>,
    arc_copy_buffer: &Arc<Vec<u8>>,
    thread_workload: &mut Vec< Vec<(usize, usize)> >,
    operations: &Vec<(SearchEngines::ComparisonOperation, T)>,
    memory_regions: &Vec<GenericOSInterface::GenericMemoryRegion>,
    thread_private_store_size: usize,
    thread_pool: &mut ThreadPool::ThreadPool<
        (
            Vec<(usize, usize)>,
            Arc<Vec<u8>>,
            usize,
            Vec<(SearchEngines::ComparisonOperation, T)>,
            fn(&[u8], usize, &[(SearchEngines::ComparisonOperation, T)], usize, &[usize]) -> Vec<usize>,
            Vec<GenericOSInterface::GenericMemoryRegion>,
            Arc<Vec<Matches::AddressMatches>>
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

        // Give the results to be later collected in order
        return thread_pool.wait_all().unwrap();
}

#[cfg(test)]
mod tests
{
    use crate::SearchRoutines::*;
    use crate::GenericOSInterface::*;
    use crate::WorkloadPartitioning::*;
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
        let num_threads: usize = 12;
        let buffer_size: usize = 1*1024*1024*1024;
        let thread_private_store_size: usize = 100000000;

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

        let operations: Vec<(SearchEngines::ComparisonOperation, u32)> = vec![(ComparisonOperation::Unequal, 1)];

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
}