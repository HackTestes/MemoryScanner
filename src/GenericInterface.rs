use crate::OSInterface;

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

pub enum GenericRegionState
{
    Resident, // It is in physical memmory
    OnlyMapped, // It only has a virtual mapping, but doesn't have any physical memory backing
    Free, // It is not mapped or stored physically in RAM
    Invalid // It returned nothing valid
}

pub struct GenericMemoryRegion
{
    pub permissions: GenericPageProtections,
    pub region_state: GenericRegionState,
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
            region_state: region_state,
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
pub struct GenericProcess
{
    handle: OSInterface::OSSpecificHandle
}

impl GenericProcess
{
    // Attach to the process
    pub fn new(process_id: u64) -> Result<Self, GenericOSErrors>
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
    pub fn get_mem_regions_info(&self) -> Result< Vec<GenericMemoryRegion>, GenericOSErrors>
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