use crate::GenericOSInterface;
use crate::Matches;

// This function helps me to merge the results from a linear search
// There are some important assumptions that if not met, will create invalid results
//    - I expect the threads results to come ordered for each region:
//        If there is a buffer of 100 elements and 4 threads, T1 holds from 0-25; T2, 25-50; T3, 50-75; T4 75-100
//    - I expect the threads to hold the regions in sequence
//        T1 holds a sequences of results from the regions in a sequential sequence: [ [region_01_results], [region_02_results], [region_03_results], ...]
// Such assumptions, make it that the first elements in each result array contain results from the same region
// Threfore I expect this:
// T1: [ [Results_R1_0-25],   [Results_R2_0-25],   [Results_R3_0-25], ... ]
// T2: [ [Results_R1_25-50],  [Results_R2_25-50],  [Results_R3_25-50], ... ]
// T3: [ [Results_R1_50-75],  [Results_R2_50-75],  [Results_R3_50-75], ... ]
// T4: [ [Results_R1_75-100], [Results_R2_75-100], [Results_R3_75-100], ... ]
// Note that such statements do not hold in an interpolated search
// &mut Vec<usize> : because I need to use the methods from it
pub fn MergeLinearSearchResults(
    results_from_threads: Vec<Vec<Vec<usize>>>,
    copied_memory_regions: &[GenericOSInterface::GenericMemoryRegion],
    num_threads: usize,
    search_results: &mut Vec<Matches::AddressMatches>)
{
        // Now merge everything in order for each of the pages copied in the buffer
        for (region_idx, region) in copied_memory_regions.iter().enumerate()
        {
            let mut region_matches: Vec<usize> = vec![];

            // Gettings the results in the same order as threads also returns ordered results
            // This is why each result index is associated to a given region
            for t_idx in 0..num_threads
            {
                region_matches.extend( &results_from_threads[t_idx][region_idx] );
            }

            // Store the result relative to all threads and append the region
            // One should only store results that exist
            if region_matches.len() != 0
            {
                search_results.push( Matches::AddressMatches::new(region.clone(), region_matches) );
            }
        }
}

// TODO: add unit testing
#[cfg(test)]
mod tests
{
    use crate::ResultMergerHelpers::*;
    use crate::GenericOSInterface::*;

    #[test]
    fn TestLinearSearchMerge()
    {
        let num_threads = 4;

        let test_copied_memory_regions = vec![
            GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 0, 100),
            GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 100, 100),
            GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 200, 100),
            GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 300, 100),
        ];

        // Consider 4 threads and 4 regions with 100 positions in the buffer
        let test_search_result = vec![
            // Thread 1
            //        region 1         region 2          region 3          region 4
            vec![ vec![0, 10, 15], vec![1, 2, 3, 4], vec![10, 11, 13], vec![5, 20] ],

            // Thread 2
            vec![ vec![25, 26, 27], vec![27, 30], vec![29, 31, 33], vec![] ],

            // Thread 3
            vec![ vec![50, 55, 60], vec![75], vec![71], vec![74, 75] ],

            // Thread 4
            vec![ vec![], vec![], vec![80], vec![] ],
        ];

        let mut merged_results = vec![];

        MergeLinearSearchResults(test_search_result, &test_copied_memory_regions, num_threads, &mut merged_results);

        let expected_buffer = vec![
            // Region 1 merged
            Matches::AddressMatches::new(test_copied_memory_regions[0].clone(), vec![0, 10, 15, 25, 26, 27, 50, 55, 60]),

            // Region 2
            Matches::AddressMatches::new(test_copied_memory_regions[1].clone(), vec![1, 2, 3, 4, 27, 30, 75]),

            // Region 3
            Matches::AddressMatches::new(test_copied_memory_regions[2].clone(), vec![10, 11, 13, 29, 31, 33, 71, 80]),

            // Region 4
            Matches::AddressMatches::new(test_copied_memory_regions[3].clone(), vec![5, 20, 74, 75])
        ];
        assert_eq!(merged_results, expected_buffer);
    }


    #[test]
    fn TestLinearSearchMerge_SingleThread()
    {
        let num_threads = 1;

        let test_copied_memory_regions = vec![
            GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 0, 100),
            GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 100, 100),
            GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 200, 100),
            GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 300, 100),
        ];

        // Consider 1 thread and 4 regions with 100 positions in the buffer
        let test_search_result = vec![
            // Thread 1
            vec![
                // Region 1
                vec![0, 10, 15, 25, 26, 27, 50, 55, 60],
                
                // Region 2
                vec![1, 2, 3, 4, 27, 30, 75],
                
                // Region 3
                vec![10, 11, 13, 29, 31, 33, 71, 80],
                
                // Region 4
                vec![]
            ],
        ];

        let mut merged_results = vec![];

        MergeLinearSearchResults(test_search_result, &test_copied_memory_regions, num_threads, &mut merged_results);

        let expected_buffer = vec![
            // Region 1 merged
            Matches::AddressMatches::new(test_copied_memory_regions[0].clone(), vec![0, 10, 15, 25, 26, 27, 50, 55, 60]),

            // Region 2
            Matches::AddressMatches::new(test_copied_memory_regions[1].clone(), vec![1, 2, 3, 4, 27, 30, 75]),

            // Region 3
            Matches::AddressMatches::new(test_copied_memory_regions[2].clone(), vec![10, 11, 13, 29, 31, 33, 71, 80]),

            // Region 4 is empty, so it doesn't show here
        ];
        assert_eq!(merged_results, expected_buffer);
    }
}