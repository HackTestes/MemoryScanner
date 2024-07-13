// Bring the generic inteface so we can translate specific to generic
use crate::GenericInterface;

use std::os::raw::c_void;

// Windows specific interfaces
#[cfg(target_os = "windows")]
use windows_sys::{
    Win32::System::Threading::*, Win32::Foundation::*,
    Win32::System::Memory::*, core::*,
    Win32::System::Diagnostics::Debug::WriteProcessMemory,
    Win32::System::Diagnostics::Debug::ReadProcessMemory,
    Win32::Foundation::HANDLE,
    Win32::Foundation::GetLastError
};

#[cfg(target_os = "windows")]
pub type OSSpecificHandle = windows_sys::Win32::Foundation::HANDLE;

// https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess
#[cfg(target_os = "windows")]
pub fn get_process_handle(process_id: u64) -> Result<windows_sys::Win32::Foundation::HANDLE, GenericInterface::GenericOSErrors>
{
    let handle = unsafe
    {
        windows_sys::Win32::System::Threading::OpenProcess(windows_sys::Win32::System::Threading::PROCESS_QUERY_INFORMATION |
        windows_sys::Win32::System::Threading::PROCESS_VM_OPERATION |
        windows_sys::Win32::System::Threading::PROCESS_VM_READ |
        windows_sys::Win32::System::Threading::PROCESS_VM_WRITE,
        0, // False
        process_id as u32) // Windows uses 32 bit PID
    };

    // If it returns NULL (0), something went wrong
    if handle == 0
    {
        return Err(GenericInterface::GenericOSErrors::GenericFail);
    }

    return Ok(handle);
}

// https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex
#[cfg(target_os = "windows")]
pub struct MemoryRegionIterator
{
    // vm stands for Virtual Memory
    // Pointer representing an address (similar to usize)
    // It should start at 0
    current_vm_address: *mut std::os::raw::c_void,
    process_handle: windows_sys::Win32::Foundation::HANDLE
}

#[cfg(target_os = "windows")]
impl Iterator for MemoryRegionIterator
{
    type Item = Result<GenericInterface::GenericMemoryRegion, GenericInterface::GenericOSErrors>;

    fn next(&mut self) -> Option< Result<GenericInterface::GenericMemoryRegion, GenericInterface::GenericOSErrors> >
    {
        // Instantiate a new memory structto hold the region's info
        let mut memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: 0,
            Type: 0,
        };

        // If the number of bytes transferred are equal to the struct size, it was a success
        let success: bool = unsafe
        {
            VirtualQueryEx(self.process_handle,
                           self.current_vm_address,
                           &mut memory_info,
                           std::mem::size_of::<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION>())
        } == std::mem::size_of::<windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION>();

        if success == true
        {
            // Align the address to the region start
            self.current_vm_address = memory_info.BaseAddress;

            // Add the size of the region, so the next query will return the next region
            self.current_vm_address = unsafe{ self.current_vm_address.add(memory_info.RegionSize) };

            // Interpret the values and make it generic
            return Some( Ok(MemoryRegionIterator::windows_translate_page_specific_to_generic(memory_info)) );
        }

        else
        {
            // We reached the end, finish the loop
            // Yeah, this function "never fails" (not a good thing, but the Windows API is a bit messy here and I could not find a workaround)
            return None
        }
    }
}

#[cfg(target_os = "windows")]
impl MemoryRegionIterator
{
    fn windows_get_page_permissions_generic(page_permission: windows_sys::Win32::System::Memory::PAGE_PROTECTION_FLAGS) -> GenericInterface::GenericPageProtections
    {
        // We have to be careful when we translate permissions to not regect valid pages
        // Remove all bits that are not related to the permission (aka ignore the rest) and then check if the perm bit remains
        // It boils down to an AND and EQUAL comparison
        // Example:
        //      0100 1001 AND 0000 0001 = 0000 0001 (current perm AND target)
        //      0000 0001 == 0000 0001
        // Note, you CANNOT combine permissions in Windows
        // https://learn.microsoft.com/en-us/windows/win32/memory/memory-protection-constants

        // Execute only
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_EXECUTE) == windows_sys::Win32::System::Memory::PAGE_EXECUTE
        {
            return GenericInterface::PageProtection_Execute;
        }

        // Execute Read
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ) == windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ
        {
            return GenericInterface::PageProtection_Execute | GenericInterface::PageProtection_Read;
        }

        // Execute Read and Write
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE) == windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE
        {
            return GenericInterface::PageProtection_Execute | GenericInterface::PageProtection_Read | GenericInterface::PageProtection_Write;
        }

        // Execute copy on Write (in other words: Read Write Execute)
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY) == windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY
        {
            return GenericInterface::PageProtection_Execute | GenericInterface::PageProtection_Read | GenericInterface::PageProtection_Write;
        }

        // Cannot access it
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_NOACCESS) == windows_sys::Win32::System::Memory::PAGE_NOACCESS
        {
            return GenericInterface::PageProtection_NoAccess;
        }

        // Read only
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_READONLY) == windows_sys::Win32::System::Memory::PAGE_READONLY
        {
            return GenericInterface::PageProtection_Read;
        }

        // Read Write
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_READWRITE) == windows_sys::Win32::System::Memory::PAGE_READWRITE
        {
            return GenericInterface::PageProtection_Read | GenericInterface::PageProtection_Write;
        }

        // Copy on Write (in other words: Read Write)
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_WRITECOPY) == windows_sys::Win32::System::Memory::PAGE_WRITECOPY
        {
            return GenericInterface::PageProtection_Read | GenericInterface::PageProtection_Write;
        }

        // Nothing worked
        return GenericInterface::PageProtection_NoAccess;

        // I am ignoring PAGE_TARGETS_INVALID and PAGE_TARGETS_NO_UPDATE
    }

    fn windows_get_region_state_generic(region_state: windows_sys::Win32::System::Memory::VIRTUAL_ALLOCATION_TYPE) -> GenericInterface::GenericRegionState
    {
        // States are mutually exclusive, but I will ignore the other bits just in case
        // Same procedure as the page protections

        // It has physical memory allocated
        if (region_state & windows_sys::Win32::System::Memory::MEM_COMMIT) == windows_sys::Win32::System::Memory::MEM_COMMIT
        {
            return GenericInterface::GenericRegionState::Resident;
        }

        // Cannot be accessed
        if (region_state & windows_sys::Win32::System::Memory::MEM_FREE) == windows_sys::Win32::System::Memory::MEM_FREE
        {
            return GenericInterface::GenericRegionState::Free;
        }

        // Only mapped, it does not have any physical memory associated yet
        if (region_state & windows_sys::Win32::System::Memory::MEM_RESERVE) == windows_sys::Win32::System::Memory::MEM_RESERVE
        {
            return GenericInterface::GenericRegionState::OnlyMapped;
        }

        // Nothing worked
        return GenericInterface::GenericRegionState::Invalid;
    }

    fn windows_translate_page_specific_to_generic(memory_region: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION) -> GenericInterface::GenericMemoryRegion
    {
        // Get permissions
        let generic_permission: GenericInterface::GenericPageProtections = Self::windows_get_page_permissions_generic(memory_region.AllocationProtect);

        // Get region state
        let generic_region_state: GenericInterface::GenericRegionState = Self::windows_get_region_state_generic(memory_region.State);

        // Get base address
        let region_base_address: usize = memory_region.BaseAddress as usize;

        // Get size
        let region_size: usize = memory_region.RegionSize;

        return GenericInterface::GenericMemoryRegion
        {
            permissions: generic_permission,
            region_state: generic_region_state,
            base_address: region_base_address,
            size_bytes: region_size
        };
    }
}

#[cfg(target_os = "windows")]
pub fn iter_over_mem_regions(handle: windows_sys::Win32::Foundation::HANDLE) -> MemoryRegionIterator
{
    // Instantiate and return the iterator
    return MemoryRegionIterator
    {
        current_vm_address: std::ptr::null_mut(),
        process_handle: handle
    };
}

// https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-writeprocessmemory
#[cfg(target_os = "windows")]
pub fn write_into_process_vm(process_handle: windows_sys::Win32::Foundation::HANDLE, buffer: &[u8], absolute_vm_address: usize) -> Result<(), GenericInterface::GenericOSErrors>
{
    let mut bytes_written: usize = 0;
    let bytes_written_ptr: *mut usize = &mut bytes_written;

    let success_code = unsafe
    {
        WriteProcessMemory(process_handle,
                           absolute_vm_address as *const c_void,
                           buffer.as_ptr() as *const c_void,
                           buffer.len(),
                           bytes_written_ptr)
    };

    // If it returns NULL (0), something went wrong
    if success_code == 0
    {
        return Err(GenericInterface::GenericOSErrors::GenericFail);
    }

    return Ok(());
}

// https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory
#[cfg(target_os = "windows")]
pub fn read_from_process_vm(process_handle: windows_sys::Win32::Foundation::HANDLE, absolute_vm_address: usize, buffer: &mut [u8]) -> Result<(), GenericInterface::GenericOSErrors>
{
    // Gets the amount of transfered bytes to the buffer
    let mut transfered_bytes: usize = 0;
    let mut transfered_bytes_ptr: *mut usize = &mut transfered_bytes;

    let success_code = unsafe
    {
        ReadProcessMemory(process_handle,
                          absolute_vm_address as *const c_void,
                          buffer.as_mut_ptr() as *mut c_void,
                          buffer.len(),
                          transfered_bytes_ptr)
    };

    // If it returns NULL (0), something went wrong
    // Was it successful?
    if success_code == 0
    {
        // No
        return Err(GenericInterface::GenericOSErrors::GenericFail);
    }

    // It was successful, but it only made a partial copy
    if (transfered_bytes != 0) && (transfered_bytes != buffer.len())
    {
        return Err(GenericInterface::GenericOSErrors::PartialReadCopy)
    }

    return Ok(());
}


// Linux specific interfaces


// MacOS specific interfaces
// MacOS won't be supported, because I don't have a Mac :) (I can't do any testing)