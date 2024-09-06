
// This conditional dependencies help me run tests in the OS dependent interfaces
// The previous method of putting everything in the same file would have no tests for type conversion
#[cfg(not(test))]
#[cfg(target_os = "windows")]
use crate::WindowsOSInterface as OSInterface;

// Use the test interface when we run tests here
#[cfg(test)]
use crate::TestOSInterface as OSInterface;

use std::fmt;

#[derive(PartialEq)]
#[derive(Debug)]
pub enum GenericOSErrors
{
    GenericFail, // If you don't want to specify the type of error (useful when you don't have good error reporting). It simply means "something went wrong"
    ProcessDoesntExist,
    PermissionDenied,
    PartialReadCopy, // Only copied part of the buffer
    SnapshotBufferIsTooSmall
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

// A type associated to the page protections
// This is done so we can have associated methods
#[derive(Debug)]
#[derive(Clone)]
#[derive(Eq, PartialEq)]
pub struct GenericPageProtectionsStruct(GenericPageProtections);

impl GenericPageProtectionsStruct
{
    pub fn new(perms: GenericPageProtections) -> GenericPageProtectionsStruct
    {
        return GenericPageProtectionsStruct(perms);
    }

    pub fn get(&self) -> GenericPageProtections
    {
        return self.0;
    }
}

impl fmt::Display for GenericPageProtectionsStruct
{
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let perms = self.get();
        let mut perms_strings: Vec<String> = Vec::new();

        // Check for no access
        if perms == PageProtection_NoAccess
        {
            // Early return
            return write!(f, "No Access");
        }
    
        // Check for read
        if (perms & PageProtection_Read) == PageProtection_Read
        {
            perms_strings.push("Read".to_string());
        }
    
        // Check for write
        if (perms & PageProtection_Write) == PageProtection_Write
        {
            perms_strings.push("Write".to_string());
        }
    
        // Check for Execute
        if (perms & PageProtection_Execute) == PageProtection_Execute
        {
            perms_strings.push("Execute".to_string());
        }
    
        return write!(f, "{}", perms_strings.join(" | "));
    }
}

// Types
// Linux:
//      - present (https://docs.kernel.org/admin-guide/mm/pagemap.html)
//      - resident (https://docs.kernel.org/filesystems/proc.html)
// Windows
//      - commited (https://learn.microsoft.com/en-us/windows/win32/memory/page-state)
#[derive(Debug)]
#[derive(Eq, PartialEq)]
#[derive(Clone)]
pub enum GenericRegionState
{
    Resident, // It is in physical memmory
    OnlyMapped, // It only has a virtual mapping, but doesn't have any physical memory backing
    Free, // It is not mapped or stored physically in RAM
    Invalid // It returned nothing valid
}

impl fmt::Display for GenericRegionState
{
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match self
        {
            GenericRegionState::Resident => return write!(f, "Resident"),
            GenericRegionState::OnlyMapped => return write!(f, "OnlyMapped"),
            GenericRegionState::Free => return write!(f, "Free"),
            GenericRegionState::Invalid => return write!(f, "Invalid")
        }
    }
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(Eq, PartialEq)]
pub struct GenericMemoryRegion
{
    pub permissions: GenericPageProtectionsStruct,
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
            permissions: GenericPageProtectionsStruct::new(page_permissions),
            state: region_state,
            base_address: page_absolute_address,
            size_bytes: size
        };
    }
}

// A fake memory region for testing
// This is just to be a name tuple
#[cfg(test)]
#[derive(Debug, PartialEq)]
pub struct FakeGenericMemoryRegion
{
    pub memory_region: GenericMemoryRegion,
    pub payload: Vec<u8> // This represents the actual memory contents of the region
}

#[cfg(test)]
impl FakeGenericMemoryRegion
{
    // This allows me to do some input validtion if I want to
    pub fn new(mem_region: GenericMemoryRegion, payload: Vec<u8>) -> Self
    {
        return FakeGenericMemoryRegion{ memory_region: mem_region, payload: payload };
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
// Note: handles should not be cloned as they can get out of sync (and one the clone might close the handle while others are using it)
#[derive(Debug)]
#[derive(PartialEq)]
pub struct GenericProcess
{
    handle: OSInterface::OSSpecificHandle,

    // An attribute to hold the custom test image from  the create method
    #[cfg(test)]
    pub custom_image: Vec< FakeGenericMemoryRegion >,
}

// Closes the handle, otherwise we will have a memory leak with the descriptors
impl Drop for GenericProcess
{
    fn drop(&mut self)
    {
        OSInterface::close_handle(self.handle);
    }
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

        #[cfg(test)]
        return Ok(GenericProcess{handle: handle, custom_image: OSInterface::default_test_process_image()});

        #[cfg(not(test))]
        return Ok(GenericProcess{handle: handle});
    }

    // A function that can only be used during tests
    // The goal of such function is to be able to create a custom fake process for each test, allowing me to test different patterns
    #[cfg(test)]
    pub fn create(process_id: u64, custom_image_input: Vec< FakeGenericMemoryRegion >) -> Self
    {
        return GenericProcess{handle: process_id, custom_image: custom_image_input};
    }

    // It also returns with pages info, nut it allows the caller to filter some desired proporties
    // page_permissions works as an at least: a page can at least read; a page can at least read and write
    // page_permissions_exact needs a perfect match: page must only have a read permision (read only)
    pub fn get_mem_regions_info(&self, page_permissions: GenericPageProtections, page_permissions_exact: Option<GenericPageProtections>, region_state: Option<GenericRegionState>) -> Result< Vec<GenericMemoryRegion>, GenericOSErrors>
    {
        // Store memory regions
        let mut memory_regions: Vec<GenericMemoryRegion> = vec![];

        for generic_memory_region_result in OSInterface::iter_over_mem_regions(self.handle, self)
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
            if (generic_memory_region.permissions.get() & page_permissions) != page_permissions
            {
                // It does not match, skip
                continue;
            }

            // It it is not empty, check if it matches
            // None matches with anything
            if (page_permissions_exact != None) && (generic_memory_region.permissions.get() != *page_permissions_exact.as_ref().unwrap())
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
        let result = OSInterface::write_into_process_vm(self.handle, buffer, absolute_vm_address, self);

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
        let result = OSInterface::read_from_process_vm(self.handle, absolute_vm_address, buffer, self);

        match result
        {
            Ok(_) => return Ok(()),
            Err(error) => return Err(error)
        };
    }

    // Builds a snapshot of the process memory based on certain regions and buffer size
    // On seccess, it resturn the amount of regions that were copied to the buffer, so the caller can ajust the parameters and retry the copy with the remaining regions
    // _bounded: it respects the limit of the buffer
    pub fn snapshot_bounded(&self, target_mem_regions: &[GenericMemoryRegion], buffer: &mut [u8]) -> Result<usize, GenericOSErrors>
    {
        let mut copies_done: usize = 0;
        let max_space = buffer.len();
        let mut space_used: usize = 0;

        for region in target_mem_regions
        {
            // Does it fit in the remaining space?
            if region.size_bytes + space_used <= max_space
            {
                // Yes, then make the copy
                // Offset the buffer by the spaced used by the other copies
                let result = self.read_from_vm(region.base_address, &mut buffer[space_used..(space_used+region.size_bytes)]);

                match result
                {
                    Ok(_) => (),
                    Err(error) => return Err(error)
                };

                // Update the control info
                copies_done += 1;
                space_used += region.size_bytes;
            }

            // The page goes out of range, no need to continue the loop
            else
            {
                break;
            }
        }

        // Check for regions too big that no copy was done
        if (target_mem_regions.len() != 0) && (copies_done == 0)
        {
            return Err(GenericOSErrors::SnapshotBufferIsTooSmall);
        }

        // DEGUG ONLY
        #[cfg(debug_print = "GOSI_snapshot_bounded")]
        {
            println!("Snapshot");
            println!("Copies done: {}", copies_done);
            println!("Space used / max size: {} / {}", space_used, max_space);
            println!("Regions copied: {:#?}", target_mem_regions[..copies_done].to_vec());
            println!("\n\n");
        }

        return Ok(copies_done);
    }

    // It returns the copy operations that will be necessary to create a snapshot and which region to start, given a buffer size
    // This also has the advantage of being able to do "look-ahead" and warn if all regions can be fit in the buffer (insted of doing it during the search)
    // Why not use an Iterator? I tried and it didn't work
    //     1. Allocating a buffer and returning it to the caller: this kills the idea of being able to
    //     reuse a buffer and remove unecessary deallocations and allocations
    //
    //     2. Borrowing a buffer: it breaks the code and doesn't allow any code to access the buffer in the loop
    //
    //     3. Arc: doesn't allow writes without UnsafeRefCell
    pub fn get_snapshot_workload(target_mem_regions: &[GenericMemoryRegion], buffer_size: usize) -> Result<Vec<usize>, GenericOSErrors>
    {
        let mut copy_operations: Vec<usize> = vec![];
        let mut copies_done: usize = 0;
        let mut space_used: usize = 0;

        let mut region_idx = 0;

        // Loop over the regions
        while region_idx < target_mem_regions.len()
        {
            let region = &target_mem_regions[region_idx];

            // Does it fit in the remaining space?
            if (region.size_bytes + space_used <= buffer_size)
            {
                // Yes
                // Only add fresh copies
                if copies_done == 0
                {
                    copy_operations.push(region_idx);
                }

                // Update the control info
                copies_done += 1;
                space_used += region.size_bytes;
                region_idx += 1;
                continue;
            }

            // It doesn't fit anymore, we need to start a new operation
            else if copies_done != 0
            {
                copies_done = 0;
                space_used = 0;

                // Replay the iteration with a "new buffer", so don't add to the region_idx
                continue;
            }

            // It didn't fit anything, therefore the buffer is too small
            else if copies_done == 0
            {
                return Err(GenericOSErrors::SnapshotBufferIsTooSmall);
            }
        }

        return Ok(copy_operations);
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
        assert_eq!( result, Ok(GenericProcess { handle: 1 , custom_image: OSInterface::default_test_process_image()}) );
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
        let operation_result = process.read_from_vm(0, &mut buffer[0..]);

        println!("Op result {:?} - buffer: {:?}", operation_result, buffer);

        // Did it succeed?
        assert!(matches!( operation_result, Ok(()) ));

        // Was the buffer written?
        assert_eq!(buffer, (0..100).collect::<Vec<u8>>());
    }

    // This is a test to see if the fake process has a predictable regions based on the vm_address
    // It should return the same pattern independently of the start address
    #[test]
    fn TestProcessReadSuccess_PredictableMemory()
    {
        // Start a zeroed buffer
        let mut buffer: Vec<u8> = vec![0; 50];

        let process = GenericProcess::attach(1).unwrap();

        // &mut buffer[0..] creates a reference slice from the vec
        let operation_result = process.read_from_vm(550, &mut buffer[0..]);

        println!("Op result {:?} - buffer: {:?}", operation_result, buffer);

        // Did it succeed?
        assert!(matches!( operation_result, Ok(()) ));

        // Was the buffer written?
        assert_eq!(buffer, (55..105).collect::<Vec<u8>>());
    }

    #[test]
    fn TestProcessPartialRead()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 100];

        //let process = GenericProcess::attach(1).unwrap();

        let process = GenericProcess::create(
            1, // PID
            vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(PageProtection_Read, GenericRegionState::Resident, 500, 100),
                    (200..250).collect()),
            ]
        );

        let operation_result = process.read_from_vm(500, &mut buffer[0..]);

        println!("Op result {:?} - buffer: {:?}", operation_result, buffer);

        // Did it succeed?
        //assert!(matches!( operation_result, Err(GenericOSErrors::PartialReadCopy) ));
        assert_eq!(operation_result, Err(GenericOSErrors::PartialReadCopy) );

        // Was the buffer written?
        let mut expected_buffer: Vec<u8> = (200..250).collect();
        expected_buffer.extend(vec![0; 50]);

        assert_eq!(buffer, expected_buffer);
    }

    #[test]
    fn TestProcessReadFail()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 100];

        let process = GenericProcess::attach(1).unwrap();

        let operation_result = process.read_from_vm(999999, &mut buffer[0..]);

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
            permision_combine = permision_combine & region.permissions.get();
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
            permision_combine = permision_combine & region.permissions.get();
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
            permision_combine = permision_combine & region.permissions.get();
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
        let process = GenericProcess::attach(6).unwrap();

        let result = process.get_mem_regions_info(PageProtection_Execute, Some(PageProtection_Read), Some(GenericRegionState::Resident));

        println!("Op result {:?}", result);

        // Did it succeed?
        assert!( matches!(result, Err(GenericOSErrors::GenericFail)) );
    }

    #[test]
    fn TestDisplayPagePermissions()
    {
        // Read only
        assert_eq!( "Read", format!("{}", GenericPageProtectionsStruct::new(PageProtection_Read)) );

        // Read Write
        assert_eq!( "Read | Write", format!("{}", GenericPageProtectionsStruct::new(PageProtection_Write | PageProtection_Read)) );

        // Read Write Execute
        assert_eq!( "Read | Write | Execute", format!("{}", GenericPageProtectionsStruct::new(PageProtection_Execute | PageProtection_Write | PageProtection_Read)) );

        // Read Execute
        assert_eq!( "Read | Execute", format!("{}", GenericPageProtectionsStruct::new(PageProtection_Execute | PageProtection_Read)) );

        // Write Execute
        assert_eq!( "Write | Execute", format!("{}", GenericPageProtectionsStruct::new(PageProtection_Execute | PageProtection_Write)) );

        // Execute only
        assert_eq!( "Execute", format!("{}", GenericPageProtectionsStruct::new(PageProtection_Execute)) );

        // Write only
        assert_eq!( "Write", format!("{}", GenericPageProtectionsStruct::new(PageProtection_Write)) );

        // No Access
        assert_eq!( "No Access", format!("{}", GenericPageProtectionsStruct::new(PageProtection_NoAccess)) );
    }

    #[test]
    fn TestDisplayResgionState()
    {
        assert_eq!("Resident", format!("{}", GenericRegionState::Resident));
        assert_eq!("Free", format!("{}", GenericRegionState::Free));
        assert_eq!("OnlyMapped", format!("{}", GenericRegionState::OnlyMapped));
        assert_eq!("Invalid", format!("{}", GenericRegionState::Invalid));
    }

    #[test]
    fn TestProcessSnapshot()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 10000];

        let process = GenericProcess::attach(1).unwrap();

        let memory_regions = process.get_mem_regions_info(PageProtection_NoAccess, None, None).unwrap();

        let snapshot_result = process.snapshot_bounded(&memory_regions[0..], &mut buffer[0..]);

        println!("Memory regions: {:?}", memory_regions);
        println!("Op result {:?} - buffer: {:?}", snapshot_result, buffer.len());

        // Did it succeed?
        assert!(matches!( snapshot_result, Ok(10) ));

        // Was the buffer written?
        let mut expect: Vec<u8> = (0..100).collect();
        expect.append(&mut (1..101).collect());
        expect.append(&mut (2..102).collect());
        expect.append(&mut (3..103).collect());
        expect.append(&mut (4..104).collect());
        expect.append(&mut (5..105).collect());
        expect.append(&mut (6..106).collect());
        expect.append(&mut (7..107).collect());
        expect.append(&mut (8..108).collect());
        expect.append(&mut (9..109).collect());
        expect.append(&mut vec![0; 9000]);

        assert_eq!(buffer, expect);
    }

    #[test]
    fn TestProcessSnapshot_BufferReuse()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 900];

        let process = GenericProcess::attach(1).unwrap();

        let memory_regions = process.get_mem_regions_info(PageProtection_NoAccess, None, None).unwrap();

        let mut snapshot_result = process.snapshot_bounded(&memory_regions[0..], &mut buffer[0..]);

        println!("Memory regions: {:?}", memory_regions);
        println!("Op result {:?} - buffer: {:?}", snapshot_result, buffer.len());

        // Did it succeed?
        assert!(matches!( snapshot_result, Ok(9) ));
        let copies_done = snapshot_result.unwrap();

        // Was the buffer written?
        let mut expect: Vec<u8> = (0..100).collect();
        expect.append(&mut (1..101).collect());
        expect.append(&mut (2..102).collect());
        expect.append(&mut (3..103).collect());
        expect.append(&mut (4..104).collect());
        expect.append(&mut (5..105).collect());
        expect.append(&mut (6..106).collect());
        expect.append(&mut (7..107).collect());
        expect.append(&mut (8..108).collect());
        assert_eq!(buffer, expect);

        // Reset buffer
        buffer.fill(0);
        assert_eq!(buffer, vec![0; 900]);

        snapshot_result = process.snapshot_bounded(&memory_regions[copies_done..], &mut buffer[0..]);

        // Did it succeed?
        assert!(matches!( snapshot_result, Ok(1) ));

        expect = (9..109).collect();
        expect.append(&mut vec![0; 800]);
        assert_eq!(buffer, expect);
    }

    #[test]
    fn TestProcessSnapshot_BufferTooSmall()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 1];

        let process = GenericProcess::attach(1).unwrap();

        let memory_regions = process.get_mem_regions_info(PageProtection_NoAccess, None, None).unwrap();

        let snapshot_result = process.snapshot_bounded(&memory_regions[0..], &mut buffer[0..]);

        println!("Memory regions: {:?}", memory_regions);
        println!("Op result {:?} - buffer: {:?}", snapshot_result, buffer.len());

        // Did it succeed?
        assert!(matches!( snapshot_result, Err(GenericOSErrors::SnapshotBufferIsTooSmall) ));
    }

    #[test]
    fn TestProcessSnapshotIter()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 500];

        let process = GenericProcess::attach(1).unwrap();

        let memory_regions = process.get_mem_regions_info(PageProtection_NoAccess, None, None).unwrap();

        let snapshot_workload = GenericProcess::get_snapshot_workload(&memory_regions[0..], buffer.len());

        let expected_work: Vec<usize> = vec![0, 5];
        assert_eq!(Ok(expected_work), snapshot_workload);
    
        for start_pos in snapshot_workload.unwrap()
        {
            let snapshot_result = process.snapshot_bounded(&memory_regions[(start_pos)..], &mut buffer[0..]);

            println!("Memory regions: {:?}", memory_regions.len());
            println!("Op result {:?} - buffer: {:?}", snapshot_result, buffer.len());

            if (snapshot_result == Err(GenericOSErrors::GenericFail)) || (snapshot_result == Ok(0))
            {
                // This shopuld happen at all with the workload calculation
                assert!(false);
            }

            // Was the buffer written?
            if start_pos == 0
            {
                let mut expect: Vec<u8> = (0..100).collect();
                expect.append(&mut (1..101).collect());
                expect.append(&mut (2..102).collect());
                expect.append(&mut (3..103).collect());
                expect.append(&mut (4..104).collect());
                assert_eq!(buffer, expect);
            }

            else if start_pos == 5
            {
                let mut expect: Vec<u8> = (5..105).collect();
                expect.append(&mut (6..106).collect());
                expect.append(&mut (7..107).collect());
                expect.append(&mut (8..108).collect());
                expect.append(&mut (9..109).collect());
                assert_eq!(buffer, expect);
            }

            else
            {
                panic!();
            }
        }
    }

    #[test]
    fn TestProcessSnapshotIterFail()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 10];

        let process = GenericProcess::attach(1).unwrap();

        let memory_regions = process.get_mem_regions_info(PageProtection_NoAccess, None, None).unwrap();

        let snapshot_workload = GenericProcess::get_snapshot_workload(&memory_regions[0..], buffer.len());

        assert_eq!(Err(GenericOSErrors::SnapshotBufferIsTooSmall), snapshot_workload);
    }

    // Previous bug detection
    #[test]
    fn TestProcessSnapshot_Bug_SkipBigPages()
    {
        let mut buffer: Vec<u8> = vec![0; 500];

        let page_perms = PageProtection_Read|PageProtection_Write;
        let page_state = GenericRegionState::Resident;

        let process = GenericProcess::create(
            1, // PID
            vec![
                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(page_perms.clone(), page_state.clone(), 100, 100),
                    vec![1; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(page_perms.clone(), page_state.clone(), 200, 500),
                    vec![2; 500]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(page_perms.clone(), page_state.clone(), 600, 100),
                    vec![3; 100]),

                FakeGenericMemoryRegion::new(
                    GenericMemoryRegion::new(page_perms.clone(), page_state.clone(), 700, 100),
                    vec![4; 100]),
            ]
        );

        let memory_regions = process.get_mem_regions_info(PageProtection_NoAccess, None, None).unwrap();

        let snapshot_result = process.snapshot_bounded(&memory_regions[0..], &mut buffer[0..]);

        println!("Memory regions: {:?}", memory_regions);
        println!("Op result {:?} - buffer: {:?}", snapshot_result, buffer.len());

        // Did it succeed?
        assert_eq!(snapshot_result, Ok(1));

        // Was the buffer written?
        let mut expect: Vec<u8> = vec![];
        expect.append(&mut vec![1; 100]);
        expect.append(&mut vec![0; 400]);

        assert_eq!(buffer, expect);
    }

}
