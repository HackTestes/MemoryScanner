# Memory sacanner

## What is it?

- Command line tool for searching and editing other processes' memory using only [ReadProcessMemory](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory) and [WriteProcessMemory](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-writeprocessmemory) APIs. Therefore, this utility will not use any form of code injection aka dll injection.

## Requirements

- WindowsOS >= 10
- Command line tool (Window terminal)

## Build

- Rust compiler
- [Windows sys](https://crates.io/crates/windows-sys)

## Important APIs and resources used

- [ReadProcessMemory](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory)

- [WriteProcessMemory](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-writeprocessmemory)

- [OpenProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess)

- [CloseHandle](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle)

- [VirtualQueryEx](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex)

- [Process Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights)

- [MEMORY_BASIC_INFORMATION](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-memory_basic_information)

- [Windows sys crate documentation](https://docs.rs/windows-sys/0.42.0/windows_sys/)

## Implementation overview

---
### Search for exact value

In this mode we know the value that we are trying to find, but we don't know the type or the location in the memory. So, we make an educated guess (integer of 4 bytes) and scan the memory in hopes of finding its address. To avoid copying the entire target process (which is quite expensive), we can filter for only pages that can be written.

After copying the necessary memory, we covert small parts to the disired type (i32 == signed integer of 4 bytes) and compare the values, furthermore we parallelize this operation.

PS: the implementation here copies a whole memory block of contiguous writable memory to avoid costs related to context switching.

---

### Search for unknown value (not done)

This one is a lot more complex, because we need a copy of all the writable memory at a certain point in time. For example, a health bar in games: we need to copy all of it at "full health" to later compare the damage. So we need all the memory for "Full Health" to later compare changes to the small chunks.

This creates a serious problem: how much memory is needed? And Can we store it? If the target is big, the system probably won't have enough memory and to mitigate this situation, we can store some results in a file simiar to [paging](https://en.wikipedia.org/wiki/Memory_paging) in OSes, but at the cost of runtime performance.

So, due to increased complexity, this mode will not be supported.

## Key features

### Read process memory
- Serach for values in memory
    - Use exact values (u32, u64, i32, i64, f32, f64)

### Write process memory
- Write process memory
    - Change values
    - Freeze values

### General
- Multithreaded search
- Minimal memory footprint
- Single executable

## Roadmap

20/11
- [ ] Read Memory
    - [ ] Multithreaded search
        - [X] Initial results
        - [ ] Search on previous results
    - [ ] Select only writable pages
    - [X] Select multiple data types
    - [ ] Copy pages (per page)
    - [ ] Copy pages (all pages at once)

25/11
- [ ] Write Memory
    - [ ] Write value on matched address
    - [ ] Freeze value

30/11
- [ ] Command parser
    - [X] Error checking
    - [X] Help
    - [ ] Save previous correct command
    - [ ] Read memory command
    - [ ] Write memory command
    - [ ] Show info on results
