// Bring the generic inteface so we can translate specific to generic
use crate::GenericOSInterface;

use std::os::raw::c_void;

// Testing dummies
// Since the OS APIs need to interface with another process, they become quite hard to test automatically
//    - It can break independent tests (we need to start a new process at every test)
//    - OS security features generally involve randomization (e.g. ASLR), making repeatable tests "impossible"

// Test handle
#[cfg(test)]
pub type OSSpecificHandle = u64;

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

#[cfg(test)]
pub fn close_handle(handle: OSSpecificHandle) -> Result<(), GenericOSInterface::GenericOSErrors>
{
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
pub struct MemoryRegionIterator
{
    // vm stands for Virtual Memory
    // Pointer representing an address (similar to usize)
    // It should start at 0
    current_vm_address: usize,
    process_handle: OSSpecificHandle
}


#[cfg(test)]
impl Iterator for MemoryRegionIterator
{
    type Item = Result<GenericOSInterface::GenericMemoryRegion, GenericOSInterface::GenericOSErrors>;

    fn next(&mut self) -> Option< Result<GenericOSInterface::GenericMemoryRegion, GenericOSInterface::GenericOSErrors> >
    {
        // Insert errors
        // It needs a special instantiator function
        if self.current_vm_address == 2000
        {
            return Some( Err(GenericOSInterface::GenericOSErrors::GenericFail) );
        }

        if self.current_vm_address < 1000
        {
            let current_vm_address = self.current_vm_address;

            // Insert pages with different permissions
            if current_vm_address == 0
            {
                // Increase the vm address, pretend that the region is of 100 bytes
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_NoAccess, GenericOSInterface::GenericRegionState::Resident, current_vm_address, 100)) );
            }

            if current_vm_address == 100
            {
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Read, GenericOSInterface::GenericRegionState::Resident, current_vm_address, 100)) );
            }

            if current_vm_address == 200
            {
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::Resident, current_vm_address, 100)) );
            }

            if current_vm_address == 300
            {
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Execute, GenericOSInterface::GenericRegionState::Resident, current_vm_address, 100)) );
            }

            if current_vm_address == 400
            {
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Execute|GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::Resident, current_vm_address, 100)) );
            }

            if current_vm_address == 500
            {
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Execute|GenericOSInterface::PageProtection_Write|GenericOSInterface::PageProtection_Read,
                                                                           GenericOSInterface::GenericRegionState::Resident,
                                                                           current_vm_address,
                                                                           100)) );
            }

            if current_vm_address == 600
            {
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::Resident, current_vm_address, 100)) );
            }

            if current_vm_address == 700
            {
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_NoAccess, GenericOSInterface::GenericRegionState::Free, current_vm_address, 100)) );
            }

            if current_vm_address == 800
            {
                self.current_vm_address += 100;
                return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::OnlyMapped, current_vm_address, 100)) );
            }

            self.current_vm_address += 100;
            return Some( Ok(GenericOSInterface::GenericMemoryRegion::new(GenericOSInterface::PageProtection_Read|GenericOSInterface::PageProtection_Write, GenericOSInterface::GenericRegionState::Resident, current_vm_address, 100)) );
        }

        else
        {
            // We reached the end, finish the loop
            return None
        }
    }
}

#[cfg(test)]
pub fn iter_over_mem_regions(handle: OSSpecificHandle) -> MemoryRegionIterator
{
    // Normal execution path
    if handle < 5
    {
        // Instantiate and return the iterator
        return MemoryRegionIterator
        {
            current_vm_address: 0,
            process_handle: handle
        };
    }

    // Needed to insert error in the memory mappings test
    else
    {
        // Instantiate and return the iterator
        return MemoryRegionIterator
        {
            current_vm_address: 2000,
            process_handle: handle
        };
    }
}


#[cfg(test)]
pub fn write_into_process_vm(process_handle: OSSpecificHandle, buffer: &[u8], absolute_vm_address: usize) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    if absolute_vm_address == 0
    {
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }

    return Ok(());
}

#[cfg(test)]
pub fn read_from_process_vm(process_handle: OSSpecificHandle, absolute_vm_address: usize, buffer: &mut [u8]) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    // Success?
    // Erros will be sent based on the input address
    if absolute_vm_address == 999999
    {
        // No
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }

    // It was successful, but it only made a partial copy
    if absolute_vm_address == 999998
    {
        // Write only to the middle to simulate a partial copy
        for i in 0..buffer.len()/2
        {
            buffer[i] = 1;
        }

        return Err(GenericOSInterface::GenericOSErrors::PartialReadCopy);
    }

    // Region identifier
    // I will use this value to be able to create a unique memory pattern for each region, so tests can catch other types of errors
    let mut region_value_id: usize = 0;

    // Do not divide by zero
    if absolute_vm_address != 0
    {
        region_value_id = absolute_vm_address/100;
    }

    // It was a success, so write into the buffer to simulate a read
    for i in 0..buffer.len()
    {
        // u8 might lose some bits of the original value,
        // but it helps to track if the region limits are being resepected
        buffer[i] = (i + region_value_id) as u8;
    }

    return Ok(());
}