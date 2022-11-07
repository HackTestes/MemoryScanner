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

## Key features

### Read process memory
- [ ] Serach for values in memory
    - [ ] Use exact values (u32, u64, i32, i64, f32, f64)
    - [ ] Use unknown initial value (Incresed, decreased or equal value)*

### Write process memory
- [ ] Write process memory

### General
- [ ] Multithreaded search
- [ ] Minimal memory footprint
- [ ] Single executable