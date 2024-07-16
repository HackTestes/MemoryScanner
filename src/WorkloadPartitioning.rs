use crate::GenericOSInterface;

// It return the positions in which the thread will iterate over
// It is important to share information about the regions, so it can in ONE loop create the thread task queue
// (insted of creating the region partitions and having to redo the iterations for the queue)
fn partition_thread_workload_equal(num_threads: usize, regions: Vec<GenericOSInterface::GenericMemoryRegion>, target_value_size_bytes: usize) -> Vec< Vec<(usize, usize)> >
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
        let segment_size: usize = region.size_bytes.div_ceil(num_threads);

        // Now iterate over each possible segment and verify if it is suitable
        for thread_id in 0..num_threads
        {
            // Each segment is based on the thread_id
            let start_pos: usize = thread_id * segment_size;

            // Does the type fit? Does the type overflow the buffer? Does it cause an out of bounds read?
            // This also prevents the end from underflowing
            if (start_pos + target_value_size_bytes) > region.size_bytes
            {
                // If so, the thread will have no work assigned
                continue;
            }

            // Calculate the last position on can read that does not cause an out of bounds read
            // +1, because the end is non inclusive
            // Otherwise a start 0 and end 0 would have no iterations
            let mut end_pos: usize = region.size_bytes - target_value_size_bytes +1;

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

// This is an alternative workload partitioning method that tries to represent more closely the actual positions that will be read
// It is particularly useful when you need to slice the original buffer and iterate over the slice.
// That means that the function used to search cannot read out of the bounds of the private segment.
// So insted of iterating over positions, it returns the positions of slice (a buffer with 10 bytes [0..10])
// It return the positions of a slice
fn partition_thread_workload_equal_slice_view(num_threads: usize, regions: Vec<GenericOSInterface::GenericMemoryRegion>, target_value_size_bytes: usize) -> Vec< Vec<(usize, usize)> >
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
        let segment_size: usize = region.size_bytes.div_ceil(num_threads);

        // Now iterate over each possible segment and verify if it is suitable
        for thread_id in 0..num_threads
        {
            // Each segment is based on the thread_id
            let start_pos: usize = thread_id * segment_size;

            // Does the type fit? Does the type overflow the buffer? Does it cause an out of bounds read?
            if (start_pos + target_value_size_bytes) > region.size_bytes
            {
                // If so, the thread will have no work assigned
                continue;
            }

            // Simply get the end of the slice, the search function will iterate over it and peferom the bounds check
            let mut end_pos: usize = region.size_bytes;

            // Does the calculation extrapolate the segment end?
            // We need to add the size of the searched value to represent the out of bound read in a slice
            // -1 is needed to stop it from reading the start search of the other thread
            // Since it walks byte per byte and can only read extra bytes of the segment, it means that it has to read at least 1 byte of his current segment
            let segment_end = start_pos + segment_size + target_value_size_bytes -1;
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

// This partitions the resutlts between all threads
// It should be used for filter operations
/*
fn partition_thread_workload_equal_slice_view(num_threads: usize, matches_addresses: Vec<Matches>) -> Vec< Vec<(usize, usize)> >
{
    
}
*/

#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::WorkloadPartitioning::*;

    fn CreateFakeMemoryRegion(size: usize) -> GenericOSInterface::GenericMemoryRegion
    {
        return GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_NoAccess, GenericOSInterface::GenericRegionState::Resident, 0, size);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_RegularCaseSingleByte()
    {
        let num_threads = 2;
        let target_size_bytes = 1;
        let memory_regions = vec![CreateFakeMemoryRegion(8)];

        let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Each thread reads 4 bytes
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,4)], vec![(4,8)] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_RegularCase()
    {
        {
            let num_threads = 2;
            let target_size_bytes = 4;
            let memory_regions = vec![CreateFakeMemoryRegion(8)];

            let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

            println!("Workload: {:?}", workload);

            // The last segment should be smaller because of the type size
            // Remember that the end index is NON INCLUSIVE and that the last position is 7 (NOT 8)
            // It reads: 4 - 5 - 6- 7 (4 bytes)
            let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,4)], vec![(4,5)] ];

            assert_eq!(expected, workload);
        }

        // Second example
        {
            let num_threads = 4;
            let target_size_bytes = 8;
            let memory_regions = vec![CreateFakeMemoryRegion(1000)];

            let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

            println!("Workload: {:?}", workload);

            let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,250)], vec![(250,500)], vec![(500,750)], vec![(750,993)] ];

            assert_eq!(expected, workload);
        }
    }

    #[test]
    fn TestWorkloadPartitioningEqual_TypeFillsTheBuffer()
    {
        let num_threads = 2;
        let target_size_bytes = 8;
        let memory_regions = vec![CreateFakeMemoryRegion(8)];

        let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Thread 0 gets to read everything in one go - avoid out of bounds read
        // Thread 1 gets no work
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,1)], vec![] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_AvoidSegmentOutOfBoundsRead()
    {
        let num_threads = 2;
        let target_size_bytes = 6;
        let memory_regions = vec![CreateFakeMemoryRegion(8)];

        let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Thread 0 needs to stope the search before his private segment ends - avoid out of bounds read
        // Pos 2 - reads current and the next 5 bytes
        // Thread 1 gets no work
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,3)], vec![] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_MoreThreadsThanMemoryPositions()
    {
        let num_threads = 6;
        let target_size_bytes = 1;
        let memory_regions = vec![CreateFakeMemoryRegion(4)];

        let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Some threads will not get some work
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,1)], vec![(1,2)], vec![(2,3)], vec![(3,4)], vec![], vec![] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_TypeTooBigSoNoSearchIsDone()
    {
        let num_threads = 4;
        let target_size_bytes = 16;
        let memory_regions = vec![CreateFakeMemoryRegion(10)];

        let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![], vec![], vec![], vec![] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_RegularCaseMultipleMemoryRegions()
    {
        let num_threads = 4;
        let target_size_bytes = 1;
        let memory_regions = vec![CreateFakeMemoryRegion(1000), CreateFakeMemoryRegion(1000)];

        let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Each thread will read 1/4 of the buffers
        // Since they are all equal, thread 1 will read 0-250 twice
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,250), (0,250)], vec![(250,500), (250,500)], vec![(500,750), (500,750)], vec![(750,1000), (750,1000)] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_RegularCaseMultipleMemoryRegionsAsymmetricalBuffers()
    {
        let num_threads = 4;
        let target_size_bytes = 8;
        let memory_regions = vec![CreateFakeMemoryRegion(1000), CreateFakeMemoryRegion(8), CreateFakeMemoryRegion(16)];

        let workload = partition_thread_workload_equal(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Each thread will read 1/4 of the buffers
        // Since they are asymmetrical, some threads will not read the second or the third buffer at all
        // Thread 0 will read all the buffers
        // Thread 1 and 2 will not read the second one
        // Thread 3 will only read the third one
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,250), (0,1), (0,4)], vec![(250,500), (4,8)], vec![(500,750), (8,9)], vec![(750,993)] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_SliceView_RegularCaseSingleByte()
    {
        let num_threads = 2;
        let target_size_bytes = 1;
        let memory_regions = vec![CreateFakeMemoryRegion(8)];

        let workload = partition_thread_workload_equal_slice_view(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // It gets a slice of size 4 bytes (this is a non inclusive slice)
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,4)], vec![(4,8)] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_SliceView_RegularCase()
    {
        let num_threads = 2;
        let target_size_bytes = 4;
        let memory_regions = vec![CreateFakeMemoryRegion(8)];

        let workload = partition_thread_workload_equal_slice_view(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // It doesn't mean that it iterates 8 time
        // It means that it will take a slice of 8 bytes (0..8) - Non inclusive
        // The function that receives the slice is the one going to decide how many iterations
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,7)], vec![(4,8)] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_SliceView_TypeFillsTheBuffer()
    {
        let num_threads = 2;
        let target_size_bytes = 8;
        let memory_regions = vec![CreateFakeMemoryRegion(8)];

        let workload = partition_thread_workload_equal_slice_view(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // It doesn't mean that it iterates 8 time
        // It means that it will take a slice of 8 bytes (0..8) - Non inclusive
        // The function that receives the slice is the one going to decide how many iterations
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,8)], vec![] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_SliceView_AvoidSegmentOutOfBoundsRead()
    {
        let num_threads = 2;
        let target_size_bytes = 6;
        let memory_regions = vec![CreateFakeMemoryRegion(8)];

        let workload = partition_thread_workload_equal_slice_view(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // It doesn't mean that it iterates 8 time
        // It means that it will take a slice of 8 bytes (0..8) - Non inclusive
        // The function that receives the slice is the one going to decide how many iterations
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,8)], vec![] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_SliceView_MoreThreadsThanMemoryPositions()
    {
        let num_threads = 6;
        let target_size_bytes = 1;
        let memory_regions = vec![CreateFakeMemoryRegion(4)];

        let workload = partition_thread_workload_equal_slice_view(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Some threads will not get some work
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,1)], vec![(1,2)], vec![(2,3)], vec![(3,4)], vec![], vec![]];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_SliceView_TypeTooBigSoNoSearchIsDone()
    {
        let num_threads = 4;
        let target_size_bytes = 16;
        let memory_regions = vec![CreateFakeMemoryRegion(10)];

        let workload = partition_thread_workload_equal_slice_view(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Some threads will not get some work
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![], vec![], vec![], vec![]];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_SliceView_RegularCaseMultipleMemoryRegions()
    {
        let num_threads = 4;
        let target_size_bytes = 1;
        let memory_regions = vec![CreateFakeMemoryRegion(1000), CreateFakeMemoryRegion(1000)];

        let workload = partition_thread_workload_equal_slice_view(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Some threads will not get some work
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,250), (0,250)], vec![(250,500), (250,500)], vec![(500,750), (500,750)], vec![(750,1000), (750,1000)] ];

        assert_eq!(expected, workload);
    }

    #[test]
    fn TestWorkloadPartitioningEqual_SliceView_RegularCaseMultipleMemoryRegionsAsymmetricalBuffers()
    {
        let num_threads = 4;
        let target_size_bytes = 8;
        let memory_regions = vec![CreateFakeMemoryRegion(1000), CreateFakeMemoryRegion(8), CreateFakeMemoryRegion(16)];

        let workload = partition_thread_workload_equal_slice_view(num_threads, memory_regions, target_size_bytes);

        println!("Workload: {:?}", workload);

        // Each thread will read 1/4 of the buffers
        // Since they are asymmetrical, some threads will not read the second or the third buffer at all
        // Thread 0 will read all the buffers
        // Thread 1 and 2 will not read the second one
        // Thread 3 will only read the third one
        let expected: Vec< Vec<(usize, usize)> > = vec![ vec![(0,257), (0,8), (0,11)], vec![(250,507), (4,15)], vec![(500,757), (8,16)], vec![(750,1000)] ];

        assert_eq!(expected, workload);
    }
}