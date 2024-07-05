// A dummy memory region struct for testing
struct MemRegion
{
    size: usize
}

impl MemRegion
{
    fn new(size_bytes: usize) -> Self
    {
        return MemRegion{size: size_bytes};
    }
}

fn partition_thread_workload_equal(num_threads: usize, regions: Vec<MemRegion>, target_value_size_bytes: usize) -> Vec< Vec<(usize, usize)> >
{
    // This holds the start and end of each thread task in private a queue
    // threads_workload_queues[0] -> thread 0 queue
    //    [(0, 10), (10, 20),...] - non inclusive end
    // threads_workload_queues[1] -> thread 1 queue
    let mut threads_workload_queues: Vec< Vec<(usize, usize)> > = vec![Vec::new(); num_threads];

    // Iterate over every region - tasks are per contiguous memory region
    for region in regions
    {
        // Calculate the segment size for each thread
        // A segment is a private memory region that the thread will use for its search
        // The value needs to be rounded to the highest integer to avoid reounding to zero (there is no size 0)
        // 0.5 -> 0 (invalid rounding)
        // 0.5 -> 1 (correct response)
        let segment_size: usize = region.size.div_ceil(num_threads);

        // Now iterate over each possible segment and verify if it is suitable
        for thread_id in 0..num_threads
        {
            // Each segment is based on the thread_id
            let start_pos: usize = thread_id * segment_size;

            // Does the type fit? Does the type overflow the buffer? Does it cause an out of bounds read?
            // This also prevents the end from underflowing
            if (start_pos + target_value_size_bytes) > region.size
            {
                // If so, the thread will have no work assigned
                continue;
            }

            // Calculate the last position on can read that does not cause an out of bounds read
            // +1, because the end is non inclusive
            // Otherwise a start 0 and end 0 would have no iterations
            let mut end_pos: usize = region.size - target_value_size_bytes +1;

            // Does the calculation extrapolate the segment end?
            let segment_end = start_pos + segment_size;
            if end_pos > segment_end
            {
                // If so, limit it to the segment end (start position + segment size)
                end_pos = segment_end;
            }

            // Assign work to each thread in its repective queue
            threads_workload_queues[thread_id].push( (start_pos, end_pos) );
        }
    }

    return threads_workload_queues;
}

#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::WorkloadPartitioning::*;

    #[test]
    fn TestWorkloadPartitioningEqual()
    {
        let num_threads = 2;
        let target_size_bytes = 4;
        let memory_regions = vec![MemRegion::new(8), MemRegion::new(1000)];

        let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        assert_eq!(true, false);
    }
}