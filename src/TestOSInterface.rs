// Bring the generic inteface so we can translate specific to generic
use crate::GenericOSInterface;



// Testing dummies
// Since the OS APIs need to interface with another process, they become quite hard to test automatically
//    - It can break independent tests (we need to start a new process at every test)
//    - OS security features generally involve randomization (e.g. ASLR), making repeatable tests "impossible"

// Test handle
#[cfg(test)]
pub type OSSpecificHandle = u64;

// A default test image process, so not every test needs to define its own process
#[cfg(test)]
pub fn default_test_process_image() -> Vec<GenericOSInterface::FakeGenericMemoryRegion>
{
    return vec![
        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_NoAccess, GenericOSInterface::GenericRegionState::Resident, 0, 100),
            (0..100).collect()),

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Read, GenericOSInterface::GenericRegionState::Resident, 100, 100),
            (1..101).collect()),

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::Resident, 200, 100),
            (2..102).collect()),

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Execute, GenericOSInterface::GenericRegionState::Resident, 300, 100),
            (3..103).collect()),

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Execute|GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::Resident, 400, 100),
            (4..104).collect()),

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Execute|GenericOSInterface::PageProtection_Write|GenericOSInterface::PageProtection_Read, GenericOSInterface::GenericRegionState::Resident, 500, 100),
            (5..105).collect()),

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::Resident, 600, 100),
            (6..106).collect()),

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_NoAccess, GenericOSInterface::GenericRegionState::Free, 700, 100),
            (7..107).collect()), // This is not correct as it should be empty, but some tests disregard this

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::OnlyMapped, 800, 100),
            (8..108).collect()),

        GenericOSInterface::FakeGenericMemoryRegion::new(
            GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::Resident, 900, 100),
            (9..109).collect()),
    ];
}

#[cfg(test)]
pub fn get_process_handle(process_id: u64) -> Result<OSSpecificHandle, GenericOSInterface::GenericOSErrors>
{
    // Errors on PID 0
    if process_id == 0
    {
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }

    return Ok(process_id);
}

// This just simulates a close handle action, but, since there is no process in tests, it does nothing
// Well, it can simulate a failure
#[cfg(test)]
pub fn close_handle(handle: OSSpecificHandle) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    // It needs a handle that can be opened, otherwise we can't drop
    if handle == 0
    {
        return Ok(());
    }

    else
    {
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }
}

#[cfg(test)]
pub fn pause_process(process: &GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    if process.pid() < 9999
    {
        return Ok(());
    }

    else
    {
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }
}

#[cfg(test)]
pub fn resume_process(process: &GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    if process.pid() < 9999
    {
        return Ok(());
    }

    else
    {
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }
}

#[cfg(test)]
pub struct MemoryRegionIterator
{
    process_handle: OSSpecificHandle,
    current_region: usize,
    regions_copy: Vec<GenericOSInterface::GenericMemoryRegion>
}

#[cfg(test)]
impl Iterator for MemoryRegionIterator
{
    type Item = Result<GenericOSInterface::GenericMemoryRegion, GenericOSInterface::GenericOSErrors>;

    fn next(&mut self) -> Option< Result<GenericOSInterface::GenericMemoryRegion, GenericOSInterface::GenericOSErrors> >
    {
        // Insert errors
        // This is here just to help testing for fails
        if self.process_handle > 5
        {
            return Some( Err(GenericOSInterface::GenericOSErrors::GenericFail) );
        }

        // We reached the end, finish the loop
        if self.current_region >= self.regions_copy.len()
        {
            return None;
        }

        // The index is valid if we get to this point, so you can simply reference the page and return a copy
        let region = Some( Ok(self.regions_copy[ self.current_region ].clone()) );
        self.current_region += 1;

        return region;
    }
}

#[cfg(test)]
pub fn iter_over_mem_regions(handle: OSSpecificHandle, process: &GenericOSInterface::GenericProcess) -> MemoryRegionIterator
{
    // Return the image of the process
    return MemoryRegionIterator
    {
        process_handle: handle,
        current_region: 0,

        // Copy the region pages into the iter struct, so it can be referenced
        // Since the pages won't cahnge, it is safe to copy
        regions_copy : process.custom_image.iter().map(|x| return x.memory_region.clone()).collect::<Vec<_>>(),
    };
}

#[cfg(test)]
pub fn write_into_process_vm(process_handle: OSSpecificHandle, buffer: &[u8], absolute_vm_address: usize, process: &GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    if absolute_vm_address == 0
    {
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }

    return Ok(());
}

#[cfg(test)]
pub fn read_from_process_vm(process_handle: OSSpecificHandle, absolute_vm_address: usize, buffer: &mut [u8], process: &GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    // Loop over every region
    for fake_region in process.custom_image.iter()
    {
        // Check if the address matches with the range
        if absolute_vm_address >= fake_region.memory_region.base_address && absolute_vm_address < (fake_region.memory_region.base_address + fake_region.memory_region.size_bytes)
        {
            // Yes, it matches.

            // Is the payload empty?
            if fake_region.payload.is_empty()
            {
                // Yes, then report the read failure
                return Err(GenericOSInterface::GenericOSErrors::GenericFail);
            }

            // Use this to ajust the indexes to the correct position in the payload
            let region_relative_idx: usize = absolute_vm_address-fake_region.memory_region.base_address;

            // Is the payload as big as the region?
            if fake_region.payload.len() == fake_region.memory_region.size_bytes
            {
                // Yes, then perform the normal copy from the selected abs addr
                for idx in 0..buffer.len()
                {
                    let payload_r = fake_region.payload.get(idx+region_relative_idx);

                    // If the buffer is out of bounds for being too big, panic
                    // Yes, a bigger buffer is valid and simply means to read the next subjacent section, but I prefer to crash and fix any ajustments in the buffer size
                    if payload_r == None {panic!("The buffer supplied for the read is too big")}
                    else {buffer[idx] = *payload_r.unwrap()}
                }

                return Ok(());
            }

            // No, then it is a partial copy (copy and report)
            else
            {
                for idx in 0..buffer.len()
                {
                    let payload_r = fake_region.payload.get(idx+region_relative_idx);

                    if payload_r == None {break;}
                    else {buffer[idx] = *payload_r.unwrap()}
                }

                return Err(GenericOSInterface::GenericOSErrors::PartialReadCopy);
            }
        }
    }

    // No region found, report as error
    return Err(GenericOSInterface::GenericOSErrors::GenericFail);
}