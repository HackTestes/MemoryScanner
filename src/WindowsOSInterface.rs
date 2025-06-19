#![allow(dead_code)]
#![cfg(target_os = "windows")]

// Bring the generic inteface so we can translate specific to generic
use crate::GenericOSInterface;

use std::os::raw::c_void;
use std::mem::size_of;

// Windows specific interfaces
#[cfg(target_os = "windows")]
use windows_sys::{
    Win32::System::Memory::*,
    Win32::System::Diagnostics::Debug::WriteProcessMemory,
    Win32::System::Diagnostics::Debug::ReadProcessMemory
};

#[cfg(target_os = "windows")]
pub type OSSpecificHandle = windows_sys::Win32::Foundation::HANDLE;

// https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess
#[cfg(target_os = "windows")]
pub fn get_process_handle(process_id: u64) -> Result<windows_sys::Win32::Foundation::HANDLE, GenericOSInterface::GenericOSErrors>
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
    if handle == std::ptr::null_mut()
    {
        eprintln!("Error from opening a process. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }

    return Ok(handle);
}

#[cfg(target_os = "windows")]
pub fn close_handle(handle: windows_sys::Win32::Foundation::HANDLE) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    let result = unsafe {windows_sys::Win32::Foundation::CloseHandle(handle)};

    if result != 0
    {
        return Ok(());
    }

    else
    {
        eprintln!("Error from closing the process handle. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }
}

#[cfg(target_os = "windows")]
pub fn pause_process(process: &GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    let result = unsafe{windows_sys::Win32::System::Diagnostics::Debug::DebugActiveProcess( process.pid() as u32 )};

    if result != 0
    {
        return Ok(());
    }

    else
    {
        eprintln!("Error from pausing the process. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }
}

#[cfg(target_os = "windows")]
pub fn resume_process(process: &GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    let result = unsafe{windows_sys::Win32::System::Diagnostics::Debug::DebugActiveProcessStop( process.pid() as u32 )};

    if result != 0
    {
        return Ok(());
    }

    else
    {
        eprintln!("Error from resuming the process. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }
}


// https://learn.microsoft.com/pt-br/windows/win32/api/psapi/nf-psapi-enumprocessmodulesex
// https://learn.microsoft.com/pt-br/windows/win32/api/psapi/nf-psapi-getmodulefilenameexa
// https://learn.microsoft.com/pt-br/windows/win32/api/psapi/ns-psapi-moduleinfo
#[allow(unused_variables)]
pub fn query_modules(handle: windows_sys::Win32::Foundation::HANDLE, process: &GenericOSInterface::GenericProcess) -> Result< Vec<GenericOSInterface::ProcessModule>, GenericOSInterface::GenericOSErrors >
{
    // The windows API uses 32 bit integers for this (should be for historical reasons)
    let mut bytes_needed: u32 = 0;
    //let bytes_needed_ptr: *mut u32 = &mut bytes_needed;

    let module_size: usize = size_of::<windows_sys::Win32::Foundation::HMODULE>();

    let mut module_buffer: Vec< windows_sys::Win32::Foundation::HMODULE > = vec![];
    
    // Get the necessary amount of bytes first
    let success_code: i32 = unsafe
    {
        windows_sys::Win32::System::ProcessStatus::EnumProcessModules(
            handle,
            module_buffer.as_mut_ptr(),
            0, // Pretend that the buffer size is zero, we don't want to write anything here yet
            &mut bytes_needed
        )
    };

    // Errors are equal to "zero" (false)
    if success_code == 0
    {
        eprintln!("Error from getting module enum buffer size. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        return Err(GenericOSInterface::GenericOSErrors::BufferError);
    }

    // Call it again with the right amount of space in the buffer

    module_buffer = vec![std::ptr::null_mut(); bytes_needed as usize/module_size];

    let success_code: i32 = unsafe
    {
        windows_sys::Win32::System::ProcessStatus::EnumProcessModules(
            handle,
            module_buffer.as_mut_ptr(),
            (module_buffer.len() * module_size) as u32,
            &mut bytes_needed
        )
    };

   if success_code == 0
    {
        eprintln!("Error from getting modules. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        return Err(GenericOSInterface::GenericOSErrors::QueryModuleError);
    }

    // Was the space sufficient? Or do they match perfectly?
    if bytes_needed as usize != module_buffer.len() * module_size
    {
        eprintln!("The buffer modules space doesn't match the bytes needed");
        return Err(GenericOSInterface::GenericOSErrors::BufferError);
    }

    // Interate over each module and get its information

    let mut final_modules: Vec<GenericOSInterface::ProcessModule> = vec![];

    for module_handle in module_buffer
    {
        // Get module name
        let mut module_name_buffer: [u16; 1024*3] = [0; 1024*3];
        let module_name_buffer_size_bytes: u32 = (module_name_buffer.len() * size_of::<u16>()) as u32;

        let module_name_success_code = unsafe
        {
            windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW(
                module_handle,
                module_name_buffer.as_mut_ptr(),
                module_name_buffer_size_bytes
            )
        };

        if module_name_success_code == 0
        {
            eprintln!("Error from getting the module name. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
            return Err(GenericOSInterface::GenericOSErrors::QueryModuleNameError);
        }

        // Everything went fine, so get the name
        let module_name: String = String::from_utf16(&module_name_buffer).unwrap();


        // Get module info (base address and size)
        let mut win_module_info: windows_sys::Win32::System::ProcessStatus::MODULEINFO = windows_sys::Win32::System::ProcessStatus::MODULEINFO
        {
            lpBaseOfDll: std::ptr::null_mut(),
            SizeOfImage: 0,
            EntryPoint: std::ptr::null_mut()
        };

        let module_info_success_code = unsafe
        {
            windows_sys::Win32::System::ProcessStatus::GetModuleInformation(
                handle,
                module_handle,
                &mut win_module_info,
                size_of::<windows_sys::Win32::System::ProcessStatus::MODULEINFO>() as u32
            )
        };

        if module_info_success_code == 0
        {
            eprintln!("Error from getting the module info. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
            return Err(GenericOSInterface::GenericOSErrors::QueryModuleNameError);
        }

        // Everything went fine, so get the module info
        let module_base_address: usize = win_module_info.lpBaseOfDll as usize;
        let module_size: usize = win_module_info.SizeOfImage as usize;

        // Create the generic module
        final_modules.push( GenericOSInterface::ProcessModule::new(
            module_name,
            module_base_address,
            module_size
        ) );
    }

    return Ok(final_modules);
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
    type Item = Result<GenericOSInterface::GenericMemoryRegion, GenericOSInterface::GenericOSErrors>;

    fn next(&mut self) -> Option< Result<GenericOSInterface::GenericMemoryRegion, GenericOSInterface::GenericOSErrors> >
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
    fn windows_get_page_permissions_generic(page_permission: windows_sys::Win32::System::Memory::PAGE_PROTECTION_FLAGS) -> GenericOSInterface::GenericPageProtections
    {
        // We have to be careful when we translate permissions to not regect valid pages
        // Remove all bits that are not related to the permission (aka ignore the rest) and then check if the perm bit remains
        // It boils down to an AND and EQUAL comparison
        // Example:
        //      0100 1001 AND 0000 0001 = 0000 0001 (current perm AND target)
        //      0000 0001 == 0000 0001
        // Note, you CANNOT combine permissions in Windows
        // https://learn.microsoft.com/en-us/windows/win32/memory/memory-protection-constants

        // Check the page_guard modifiers as they cannot be accessed and can fire memory violations (even from ReadProcessMemory)
        // Consider them No access
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_GUARD) == windows_sys::Win32::System::Memory::PAGE_GUARD
        {
            return GenericOSInterface::PageProtection_NoAccess;
        }

        // Consider them No access
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_WRITECOMBINE) == windows_sys::Win32::System::Memory::PAGE_WRITECOMBINE
        {
            return GenericOSInterface::PageProtection_NoAccess;
        }

        // Execute only
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_EXECUTE) == windows_sys::Win32::System::Memory::PAGE_EXECUTE
        {
            return GenericOSInterface::PageProtection_Execute;
        }

        // Execute Read
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ) == windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ
        {
            return GenericOSInterface::PageProtection_Execute | GenericOSInterface::PageProtection_Read;
        }

        // Execute Read and Write
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE) == windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE
        {
            return GenericOSInterface::PageProtection_Execute | GenericOSInterface::PageProtection_Read | GenericOSInterface::PageProtection_Write;
        }

        // Execute copy on Write (in other words: Read Write Execute)
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY) == windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY
        {
            return GenericOSInterface::PageProtection_Execute | GenericOSInterface::PageProtection_Read | GenericOSInterface::PageProtection_Write;
        }

        // Cannot access it
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_NOACCESS) == windows_sys::Win32::System::Memory::PAGE_NOACCESS
        {
            return GenericOSInterface::PageProtection_NoAccess;
        }

        // Read only
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_READONLY) == windows_sys::Win32::System::Memory::PAGE_READONLY
        {
            return GenericOSInterface::PageProtection_Read;
        }

        // Read Write
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_READWRITE) == windows_sys::Win32::System::Memory::PAGE_READWRITE
        {
            return GenericOSInterface::PageProtection_Read | GenericOSInterface::PageProtection_Write;
        }

        // Copy on Write (in other words: Read Write)
        if (page_permission & windows_sys::Win32::System::Memory::PAGE_WRITECOPY) == windows_sys::Win32::System::Memory::PAGE_WRITECOPY
        {
            return GenericOSInterface::PageProtection_Read | GenericOSInterface::PageProtection_Write;
        }

        // Nothing worked
        return GenericOSInterface::PageProtection_NoAccess;

        // I am ignoring PAGE_TARGETS_INVALID and PAGE_TARGETS_NO_UPDATE
    }

    fn windows_get_region_state_generic(region_state: windows_sys::Win32::System::Memory::VIRTUAL_ALLOCATION_TYPE) -> GenericOSInterface::GenericRegionState
    {
        // States are mutually exclusive, but I will ignore the other bits just in case
        // Same procedure as the page protections

        // It has physical memory allocated
        if (region_state & windows_sys::Win32::System::Memory::MEM_COMMIT) == windows_sys::Win32::System::Memory::MEM_COMMIT
        {
            return GenericOSInterface::GenericRegionState::Resident;
        }

        // Cannot be accessed
        if (region_state & windows_sys::Win32::System::Memory::MEM_FREE) == windows_sys::Win32::System::Memory::MEM_FREE
        {
            return GenericOSInterface::GenericRegionState::Free;
        }

        // Only mapped, it does not have any physical memory associated yet
        if (region_state & windows_sys::Win32::System::Memory::MEM_RESERVE) == windows_sys::Win32::System::Memory::MEM_RESERVE
        {
            return GenericOSInterface::GenericRegionState::OnlyMapped;
        }

        // Nothing worked
        return GenericOSInterface::GenericRegionState::Invalid;
    }

    fn windows_translate_page_specific_to_generic(memory_region: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION) -> GenericOSInterface::GenericMemoryRegion
    {
        // Get the current permissions, careful to not use AllocationProtect (this represents the INITIAL permissions)!
        let generic_permission: GenericOSInterface::GenericPageProtections = Self::windows_get_page_permissions_generic(memory_region.Protect);

        // Get region state
        let generic_region_state: GenericOSInterface::GenericRegionState = Self::windows_get_region_state_generic(memory_region.State);

        // Get base address
        let region_base_address: usize = memory_region.BaseAddress as usize;

        // Get size
        let region_size: usize = memory_region.RegionSize;

        return GenericOSInterface::GenericMemoryRegion::new(generic_permission, generic_region_state, region_base_address, region_size);
    }
}

// This is done so the caller inly needs to call one function to be able to iterate over mem regions
#[allow(unused_variables)] // TODO: IMPROVE THE API
#[cfg(target_os = "windows")]
pub fn iter_over_mem_regions(handle: windows_sys::Win32::Foundation::HANDLE, process: &GenericOSInterface::GenericProcess) -> MemoryRegionIterator
{
    // Instantiate and return the iterator
    return MemoryRegionIterator
    {
        current_vm_address: std::ptr::null_mut(),
        process_handle: handle
    };
}

// https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-writeprocessmemory
#[allow(unused_variables)] // TODO: IMPROVE THE API
#[cfg(target_os = "windows")]
pub fn write_into_process_vm(process_handle: windows_sys::Win32::Foundation::HANDLE, buffer: &[u8], absolute_vm_address: usize, process: &GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
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
        eprintln!("Error from write. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }

    return Ok(());
}

// https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory
#[allow(unused_variables)] // TODO: IMPROVE THE API
#[cfg(target_os = "windows")]
pub fn read_from_process_vm(process_handle: windows_sys::Win32::Foundation::HANDLE, absolute_vm_address: usize, buffer: &mut [u8], process: &GenericOSInterface::GenericProcess) -> Result<(), GenericOSInterface::GenericOSErrors>
{
    // Gets the amount of transfered bytes to the buffer
    let mut transfered_bytes: usize = 0;
    let transfered_bytes_ptr: *mut usize = &mut transfered_bytes;

    let success_code = unsafe
    {
        ReadProcessMemory(process_handle,
                          absolute_vm_address as *const c_void,
                          buffer.as_mut_ptr() as *mut c_void,
                          buffer.len(),
                          transfered_bytes_ptr)
    };

    //println!("Process_handle: {} \nAbsolute VM addr: {} \nBuffer ptr: {} \nBuffer len: {} \nTrasferred bytes: {}\n\n", process_handle, absolute_vm_address, buffer.as_mut_ptr() as usize, buffer.len(), transfered_bytes);

    // If it returns NULL (0), something went wrong
    // Was it successful?
    if success_code == 0
    {
        use std::mem::size_of;
        // No
        eprintln!("Error from read. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        eprintln!("Page: {:#?}", (format!("{:#01$X}", absolute_vm_address, size_of::<usize>() * 2 + 2), buffer.len(), transfered_bytes));
        return Err(GenericOSInterface::GenericOSErrors::GenericFail);
    }

    // It was successful, but it only made a partial copy
    if (transfered_bytes != 0) && (transfered_bytes != buffer.len())
    {
        eprintln!("Error from read. Windows error code: {}", unsafe{windows_sys::Win32::Foundation::GetLastError()});
        return Err(GenericOSInterface::GenericOSErrors::PartialReadCopy)
    }

    return Ok(());
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests
{
    use crate::WindowsOSInterface::*;

    #[test]
    fn TestWindows_PageStateConversion_Resident()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: windows_sys::Win32::System::Memory::MEM_COMMIT,
            Protect: 0,
            Type: 0,
        };

        let generic_memory_info_state = MemoryRegionIterator::windows_get_region_state_generic(memory_info.State);

        println!("{}", generic_memory_info_state);

        assert_eq!(GenericOSInterface::GenericRegionState::Resident, generic_memory_info_state);
    }

    #[test]
    fn TestWindows_PageStateConversion_Free()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: windows_sys::Win32::System::Memory::MEM_FREE,
            Protect: 0,
            Type: 0,
        };

        let generic_memory_info_state = MemoryRegionIterator::windows_get_region_state_generic(memory_info.State);

        println!("{}", generic_memory_info_state);

        assert_eq!(GenericOSInterface::GenericRegionState::Free, generic_memory_info_state);
    }

    #[test]
    fn TestWindows_PageStateConversion_OnlyMapped()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: windows_sys::Win32::System::Memory::MEM_RESERVE,
            Protect: 0,
            Type: 0,
        };

        let generic_memory_info_state = MemoryRegionIterator::windows_get_region_state_generic(memory_info.State);

        println!("{}", generic_memory_info_state);

        assert_eq!(GenericOSInterface::GenericRegionState::OnlyMapped, generic_memory_info_state);
    }

    #[test]
    fn TestWindows_PageStateConversion_Invalid()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 10,
            Protect: 0,
            Type: 0,
        };

        let generic_memory_info_state = MemoryRegionIterator::windows_get_region_state_generic(memory_info.State);

        println!("{}", generic_memory_info_state);

        assert_eq!(GenericOSInterface::GenericRegionState::Invalid, generic_memory_info_state);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_ExecuteOnly()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_EXECUTE,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_Execute, generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_ExecuteRead()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_Execute | GenericOSInterface::PageProtection_Read, generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_ExecuteReadWrite()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_Execute | GenericOSInterface::PageProtection_Read | GenericOSInterface::PageProtection_Write, generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_ReadOnly()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_READONLY,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_Read, generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_ReadWrite()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_READWRITE,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_Read | GenericOSInterface::PageProtection_Write, generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_COW_ExecuteReadWrite()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_Execute | GenericOSInterface::PageProtection_Read | GenericOSInterface::PageProtection_Write, generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_COW_ReadWrite()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_WRITECOPY,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_Read | GenericOSInterface::PageProtection_Write, generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_NoAccess()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_NOACCESS,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_NoAccess , generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_PagePermissionsConversion_ReadOnly_IgnoreRest()
    {
        // Instantiate a new memory structto hold the region's info
        let memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            PartitionId: 0,
            State: 0,
            Protect: windows_sys::Win32::System::Memory::PAGE_READONLY | windows_sys::Win32::System::Memory::PAGE_NOCACHE,
            Type: 0,
        };

        let generic_memory_info_perms = MemoryRegionIterator::windows_get_page_permissions_generic(memory_info.Protect);

        println!("{}", generic_memory_info_perms);

        assert_eq!(GenericOSInterface::PageProtection_Read , generic_memory_info_perms);
    }

    #[test]
    fn TestWindows_SpecificToGenericCoversion()
    {
        // Instantiate a new memory structto hold the region's info
        let mut memory_info: windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION = windows_sys::Win32::System::Memory::MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 10240,
            PartitionId: 0,
            State: windows_sys::Win32::System::Memory::MEM_COMMIT,
            Protect: windows_sys::Win32::System::Memory::PAGE_READONLY | windows_sys::Win32::System::Memory::PAGE_NOCACHE,
            Type: 0,
        };

        memory_info.BaseAddress = unsafe{ memory_info.BaseAddress.add(2048) };

        let generic_memory_info = MemoryRegionIterator::windows_translate_page_specific_to_generic(memory_info);

        println!("{:?}", generic_memory_info);

        let expected_generic_mem_region = GenericOSInterface::GenericMemoryRegion::new(
                                                                                    GenericOSInterface::PageProtection_Read,
                                                                                    GenericOSInterface::GenericRegionState::Resident,
                                                                                    2048,
                                                                                    10240
        );
        assert_eq!(expected_generic_mem_region , generic_memory_info);
    }
}