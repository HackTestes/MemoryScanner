use crate::OSInterface;

#[derive(Debug)]
pub enum GenericOSErrors
{
    GenericFail, // If you don't want to specify the type of error (useful when you don't have good error reporting). It simply means "something went wrong"
    ProcessDoesntExist,
    PermissionDenied,
    PartialReadCopy // Only copied part of the buffer
}

// Each bit represent a specific permission
// 32 bits might be overkill (as it can store up to 32 permissions), but I would like to future proof the API a little bit
// I am using a bit map to facilitate the usage, such as composing multiple permissions in a single variable
// It also eases the use of bit operations (AND, OR, XOR...)
pub type GenericPageProtections = u32;

pub const PageProtection_NoAccess: u32 = 0b00000000_00000000_00000000_00000000_u32; // Redundant, but can be useful for some OSes
pub const PageProtection_Read: u32 =     0b00000000_00000000_00000000_00000001_u32;
pub const PageProtection_Write: u32 =    0b00000000_00000000_00000000_00000010_u32;
pub const PageProtection_Execute: u32 =  0b00000000_00000000_00000000_00000100_u32;


// Types
// Linux:
//      - present (https://docs.kernel.org/admin-guide/mm/pagemap.html)
//      - resident (https://docs.kernel.org/filesystems/proc.html)
// Windows
//      - commited (https://learn.microsoft.com/en-us/windows/win32/memory/page-state)
#[derive(Debug)]
#[derive(PartialEq)]
pub enum GenericRegionState
{
    Resident, // It is in physical memmory
    OnlyMapped, // It only has a virtual mapping, but doesn't have any physical memory backing
    Free, // It is not mapped or stored physically in RAM
    Invalid // It returned nothing valid
}

#[derive(Debug)]
pub struct GenericMemoryRegion
{
    pub permissions: GenericPageProtections,
    pub state: GenericRegionState,
    pub base_address: usize, // Read it as the absolute virtual addresss in the target virtual space
    pub size_bytes: usize
}

impl GenericMemoryRegion
{
    pub fn new(page_permissions: GenericPageProtections, region_state: GenericRegionState, page_absolute_address: usize, size: usize) -> Self
    {
        return GenericMemoryRegion
        {
            permissions: page_permissions,
            state: region_state,
            base_address: page_absolute_address,
            size_bytes: size
        };
    }
}

// It represents a single ATTACHED process
// Attaching early is important to avoid process ID race conditions
// Example: you pass PID 50 and attach to it
// -> you read PID 50 from the handle
// -> PID 50 exits and something else becomes PID 50
// -> you reattach to PID 50 (but now it is a different process)
// -> you make a nes scan and get bad results
// A handle would avoid this problem entirely
#[derive(Debug)]
pub struct GenericProcess
{
    handle: OSInterface::OSSpecificHandle
}

impl GenericProcess
{
    // Attach to the process
    pub fn attach(process_id: u64) -> Result<Self, GenericOSErrors>
    {
        // The OSInterface should deal with the process opening details
        let result = OSInterface::get_process_handle(process_id);

        // Check for erros
        let handle = match result
        {
            Ok(handle_unpacked) => handle_unpacked,
            Err(error) => return Err(error)
        };

        return Ok(GenericProcess{handle: handle});
    }

    // It also returns with pages info, nut it allows the caller to filter some desired proporties
    // page_permissions works as an at least: a page can at least read; a page can at least read and write
    // page_permissions_exact needs a perfect match: page must only have a read permision (read only)
    pub fn get_mem_regions_info(&self, page_permissions: GenericPageProtections, page_permissions_exact: Option<GenericPageProtections>, region_state: Option<GenericRegionState>) -> Result< Vec<GenericMemoryRegion>, GenericOSErrors>
    {
        // Store memory regions
        let mut memory_regions: Vec<GenericMemoryRegion> = vec![];

        for generic_memory_region_result in OSInterface::iter_over_mem_regions(self.handle)
        {
            // Check for errors
            let generic_memory_region = match generic_memory_region_result
            {
                Ok(mem_region) => mem_region,
                Err(error) => return Err(error)
            };

            // At least: ignore the rest of the bits and check if the result matches the target
            // 1001 0001 AND 0000 0011 = 0000 0001
            // 0000 0001 == 0000 0011 -> does not match
            // NoAccess matches with everything
            if (generic_memory_region.permissions & page_permissions) != page_permissions
            {
                // It does not match, skip
                continue;
            }

            // It it is not empty, check if it matches
            // None matches with anything
            if (page_permissions_exact != None) && (generic_memory_region.permissions != *page_permissions_exact.as_ref().unwrap())
            {
                // It does not match, skip
                continue;
            }

            // It it is not empty, check if it matches
            // None matches with anything
            // State can only have an exact match (you can't be free and resident at the same time)
            if (region_state != None) && (generic_memory_region.state != *region_state.as_ref().unwrap())
            {
                // It does not match, skip
                continue;
            }

            // Everything is ok, store it
            memory_regions.push(generic_memory_region);
        }
        return Ok(memory_regions);
    }

    // vm stands for Virtual Memory
    // source -> destination
    // this is not unsafe relative to the caller, but it can definitely currupt the target
    pub fn write_into_vm(&self, buffer: &[u8], absolute_vm_address: usize) -> Result<(), GenericOSErrors>
    {
        // The OSInterface function is responsible for understanding how to use the handle
        let result = OSInterface::write_into_process_vm(self.handle, buffer, absolute_vm_address);

        match result
        {
            Ok(_) => return Ok(()),
            Err(error) => return Err(error)
        };
    }

    // vm stands for Virtual Memory
    // source -> destination
    // There is no need to pass how many bytes will be copied, the function will derive it from the buffer length (or slice)
    pub fn read_from_vm(&self, absolute_vm_address: usize, buffer: &mut [u8]) -> Result<(), GenericOSErrors>
    {
        // The OSInterface function is responsible for understanding how to use the handle
        let result = OSInterface::read_from_process_vm(self.handle, absolute_vm_address, buffer);

        match result
        {
            Ok(_) => return Ok(()),
            Err(error) => return Err(error)
        };
    }
}


#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::GenericOSInterface::*;

    // Does the attach method check for errors and return the handle on success?
    #[test]
    fn TestProcessAttachSuccess()
    {
        let result = GenericProcess::attach(1);
        println!("{:?}", result);

        // Does it return a handle?
        assert!( matches!( result, Ok(GenericProcess { handle: 1 }) ) );
    }

    // Does the attach method check for errors and return the handle on success?
    #[test]
    fn TestProcessAttachError()
    {
        let result = GenericProcess::attach(0);
        println!("{:?}", result);

        // Does it return the error?
        assert!( matches!( result, Err(GenericOSErrors::GenericFail) ) );
    }

    #[test]
    fn TestProcessReadSuccess()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 100];

        let process = GenericProcess::attach(1).unwrap();

        // &mut buffer[0..] creates a reference slice from the vec
        let operation_result = process.read_from_vm(2, &mut buffer[0..]);

        println!("Op result {:?} - buffer: {:?}", operation_result, buffer);

        // Did it succeed?
        assert!(matches!( operation_result, Ok(()) ));

        // Was the buffer written?
        assert_eq!(buffer, vec![1; 100]);
    }

    #[test]
    fn TestProcessPartialRead()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 100];

        let process = GenericProcess::attach(1).unwrap();

        let operation_result = process.read_from_vm(1, &mut buffer[0..]);

        println!("Op result {:?} - buffer: {:?}", operation_result, buffer);

        // Did it succeed?
        assert!(matches!( operation_result, Err(GenericOSErrors::PartialReadCopy) ));

        // Was the buffer written?
        let mut expected_buffer = vec![1; 50];
        expected_buffer.extend(vec![0; 50]);

        assert_eq!(buffer, expected_buffer);
    }

    #[test]
    fn TestProcessReadFail()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 100];

        let process = GenericProcess::attach(1).unwrap();

        let operation_result = process.read_from_vm(0, &mut buffer[0..]);

        println!("Op result {:?} - buffer: {:?}", operation_result, buffer);

        // Did it succeed?
        assert!(matches!( operation_result, Err(GenericOSErrors::GenericFail) ));

        // Was the buffer written?
        assert_eq!(buffer, vec![0; 100]);
    }

    #[test]
    fn TestProcessWriteSuccess()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 10];

        let process = GenericProcess::attach(1).unwrap();

        let operation_result = process.write_into_vm(&mut buffer[0..], 1);

        println!("Op result {:?} - buffer: {:?}", operation_result, buffer);

        // Did it succeed?
        assert!(matches!( operation_result, Ok(()) ));
    }

    #[test]
    fn TestProcessWriteFail()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 10];

        let process = GenericProcess::attach(1).unwrap();

        let operation_result = process.write_into_vm(&mut buffer[0..], 0);

        println!("Op result {:?} - buffer: {:?}", operation_result, buffer);

        // Did it succeed?
        assert!(matches!( operation_result, Err(GenericOSErrors::GenericFail) ));
    }

    #[test]
    fn TestProcessVMMappingsSuccess_NoFilter()
    {
        let process = GenericProcess::attach(1).unwrap();

        let result = process.get_mem_regions_info(PageProtection_NoAccess, None, None);

        println!("Op result {:?}", result);

        // Did it succeed?
        assert!( matches!(result, Ok(_)) );

        let result_value = result.unwrap();

        // Does it have all of the mappings?
        assert_eq!( result_value.len(), 10 );
    }

    #[test]
    fn TestProcessVMMappingsSuccess_FilterPermAtLeast()
    {
        let process = GenericProcess::attach(1).unwrap();

        let result = process.get_mem_regions_info(PageProtection_Read, None, None);

        println!("Op result {:?}", result);

        // Did it succeed?
        assert!( matches!(result, Ok(_)) );

        let result_value = result.unwrap();

        // Does it have all of the correct mappings?
        assert_eq!( result_value.len(), 4 );

        // Combine all permissions and see if they still have the read bit with an AND
        let mut permision_combine = PageProtection_Read;
        for region in result_value
        {
            permision_combine = permision_combine & region.permissions;
        }

        assert_eq!( PageProtection_Read, permision_combine);
    }

    #[test]
    fn TestProcessVMMappingsSuccess_FilterPermExact()
    {
        let process = GenericProcess::attach(1).unwrap();

        let result = process.get_mem_regions_info(PageProtection_NoAccess, Some(PageProtection_Read), None);

        println!("Op result {:?}", result);

        // Did it succeed?
        assert!( matches!(result, Ok(_)) );

        let result_value = result.unwrap();

        // Does it have all of the correct mappings?
        assert_eq!( result_value.len(), 1 );

        // Combine all permissions and see if they still have the read bit with an AND
        let mut permision_combine = PageProtection_Read;
        for region in result_value
        {
            permision_combine = permision_combine & region.permissions;
        }

        assert_eq!( PageProtection_Read, permision_combine);
    }

    #[test]
    fn TestProcessVMMappingsSuccess_FilterState()
    {
        let process = GenericProcess::attach(1).unwrap();

        let result = process.get_mem_regions_info(PageProtection_NoAccess, None, Some(GenericRegionState::Free));

        println!("Op result {:?}", result);

        // Did it succeed?
        assert!( matches!(result, Ok(_)) );

        let result_value = result.unwrap();

        // Does it have all of the correct mappings?
        assert_eq!( result_value.len(), 1 );

        for region in result_value
        {
            //Do they all have the right state
            assert_eq!(GenericRegionState::Free, region.state);
        }
    }

    #[test]
    fn TestProcessVMMappingsSuccess_FilterCombination()
    {
        let process = GenericProcess::attach(1).unwrap();

        // It makes no sense to combine the at least match with the exact one
        let result = process.get_mem_regions_info(PageProtection_Execute, None, Some(GenericRegionState::Resident));

        println!("Op result {:?}", result);

        // Did it succeed?
        assert!( matches!(result, Ok(_)) );

        let result_value = result.unwrap();

        // Does it have all of the correct mappings?
        assert_eq!( result_value.len(), 3 );

        for region in &result_value
        {
            //Do they all have the right state
            assert_eq!(GenericRegionState::Resident, region.state);
        }

        // Combine all permissions and see if they still have the read bit with an AND
        let mut permision_combine = PageProtection_Execute;
        for region in &result_value
        {
            permision_combine = permision_combine & region.permissions;
        }
        assert_eq!( PageProtection_Execute, permision_combine);
    }

    #[test]
    fn TestProcessVMMappingsSuccess_FilterPermCombination()
    {
        let process = GenericProcess::attach(1).unwrap();

        // It makes no sense to combine the at least match with the exact one
        // This example is just to show one of such incompatible cases
        // Searches for a region that can be at least executed and read-only
        let result = process.get_mem_regions_info(PageProtection_Execute, Some(PageProtection_Read), Some(GenericRegionState::Resident));

        println!("Op result {:?}", result);

        // Did it succeed?
        assert!( matches!(result, Ok(_)) );

        let result_value = result.unwrap();

        // Does it have all of the correct mappings?
        // In this case, it should be empty because of the filters
        assert_eq!( result_value.len(), 0 );
    }

    #[test]
    fn TestProcessVMMappingsFail()
    {
        let process = GenericProcess::attach(1).unwrap();

        // It makes no sense to combine the at least match with the exact one
        // This example is just to show one of such incompatible cases
        // Searches for a region that can be at least executed and read-only
        let result = process.get_mem_regions_info(PageProtection_Execute, Some(PageProtection_Read), Some(GenericRegionState::Resident));

        println!("Op result {:?}", result);

        todo!();

        // Did it succeed?
        assert!( matches!(result, Err(GenericOSErrors::GenericFail)) );

    }
}
