use crate::OSInterface;
use std::fmt;

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
#[derive(PartialEq)]
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

    // Builds a snapshot of the process memory based on certain regions and buffer size
    // On seccess, it resturn the amount of regions that were copied to the buffer, so the caller can ajust the parameters and retry the copy with the remaining regions
    // _bounded: it respects the limit of the buffer
    pub fn snapshot_bounded(&self, target_mem_regions: &[GenericMemoryRegion], buffer: &mut [u8]) -> Result<usize, GenericOSErrors>
    {
        let mut copies_done: usize = 0;
        let max_space = buffer.len();
        let mut space_used: usize = 0;

        // Only iterate over the REAMAINING regions (so offset it by the start pos)
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
        }

        // Check for regions too big that no copy was done
        if (target_mem_regions.len() != 0) && (copies_done == 0)
        {
            return Err(GenericOSErrors::SnapshotBufferIsTooSmall);
        }

        return Ok(copies_done);
    }
}


#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::GenericOSInterface::*;
    use crate::OSInterface::*;

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
        assert_eq!(buffer, (0..100).collect::<Vec<u8>>());
    }

    #[test]
    fn TestProcessPartialRead()
    {
        // Start a zeroed buffer of 100 items
        let mut buffer: Vec<u8> = vec![0; 100];

        let process = GenericProcess::attach(1).unwrap();

        let operation_result = process.read_from_vm(999998, &mut buffer[0..]);

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
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
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
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        expect.append(&mut (0..100).collect());
        assert_eq!(buffer, expect);

        // Reset buffer
        buffer.fill(0);
        assert_eq!(buffer, vec![0; 900]);

        snapshot_result = process.snapshot_bounded(&memory_regions[copies_done..], &mut buffer[0..]);

        // Did it succeed?
        assert!(matches!( snapshot_result, Ok(1) ));

        expect = (0..100).collect();
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
}
