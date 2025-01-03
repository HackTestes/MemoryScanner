# Memory scanner

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

## Usage


```powershell
MemoryScanner.exe <PROCESS_ID>
```

You can use 4 commands: search, write, display and exit. Use --help to display the help text.
```
command [OPTIONS]
```

## Implementation overview

---
### Search for exact value

In this mode we know the value that we are trying to find, but we don't know the type or the location in the memory. So, we make an educated guess (integer of 4 bytes) and scan the memory in hopes of finding its address. To avoid copying the entire target process (which is quite expensive), we can filter for only pages that can be written.

After copying the necessary memory, we covert small parts to the desired type (i32 == signed integer of 4 bytes) and compare the values, furthermore we parallelize this operation.

PS: the implementation here copies a whole memory block of contiguous writable memory to avoid costs related to context switching.

---

### Search for unknown value (not done)

This one is a lot more complex, because we need a copy of all the writable memory at a certain point in time. For example, a health bar in games: we need to copy all of it at "full health" to later compare the damage. So we need all the memory for "Full Health" to later compare changes to the small chunks.

This creates a serious problem: how much memory is needed? And Can we store it? If the target is big, the system probably won't have enough memory and to mitigate this situation, we can store some results in a file similar to [paging](https://en.wikipedia.org/wiki/Memory_paging) in OSes, but at the cost of runtime performance.

So, due to increased complexity, this mode will not be supported.

## Key features

### Read process memory
- Search for values in memory
    - Use exact values (u8, u16, u32, u64, i32, i64, f32, f64)

### Write process memory
- Write process memory
    - Change values
    - Freeze values

### General
- Multithreaded search
- Minimal memory footprint
- Single executable

## Debugging

### Set up debug printing information

Select rust flags (Windows). The flags are only valid for that terminal session
```
$env:RUSTFLAGS='--cfg flag_name="flag value"'

OR

$env:RUSTFLAGS='--cfg flag_name="flag value" --cfg flag_name="flag value 02"'

OR

$env:RUSTFLAGS='--cfg flag_name'
```

Then you just need to build and run
```
cargo build [--release]
```

## Roadmap

* Just refactoring work! It's fully working now!

* [x] Implementation for a thread pool (performance optimization)

* [x] Find a way to reuse search functions (start and filter)

* Refactoring

    * [x] Update argument parsing
        * Goal: improve readability, because the current code looks terrible (it was an experiment at different ways of doing argument parsing)
        * How: get rid of the hash map, get rid of actions, use match or if-else and make a arguments struct that holds everything (it should help that only 1 struct holds all of the data insted of multiple places)

    * [x] Refactor parallel search 
        * Goal: make the algorithm simplier
        * Current situation: I made some bad decisions on the algorithm (making it hard and unreadable tbh), but I have since refined it
        * How: update the docs first and then updatae the code

    * [x] Make the parallel search code a routine, to improve on testing and improve readability of the Glue code

* Performance optimization

    * [x] Use threads pools in the search
        * Goal: avoid the costs associated to thread creation at every region search (it currently creates threads at every new region)

    * [ ] Use vectorized instructions for searching
        * Goal: improve the speed of the search hot path
        * How: Rust's regex engine uses such instructions, so I will use the binary search version (https://github.com/BurntSushi/memchr)

    * [x] Run the same code for multiple regions
        * Goal: better exploit the code cache in CPUs to improve speed
        * How: store multiple regions in a single structure that keeps track of the memory used (size), so the search code will be reutilized withut calling OS APIs and will make it reside in CPUs cache
        * Sub-goal: make the regions contiguous in memory, allow both the code and data caches to be better exploited

    * [ ] Reduce memory usage
        * Goal: memory allocations can be slow
        * How: remove buffer cloning where possible in the codebase. I might be able to acheive it by using a scoped thread pool implementation and share slices
        
    * [ ] Add IDs to regions
        * Goal: I don't remember anymore, but I suspect it has to do with better management of pages (aka access them directly in a hash map)
        * How: we can create an ID based on the base address and the region size

* Stability and correctness

    * [ ] Add more unit tests
        * Goal: Detect the behaviour of misaligned filter searches (the global match page ajustment)
        * How: Yet to be investigated

* Features

    * [ ] QoL:Add a configuration to ignore read page errors or add one to halt if wanted (so stop by default)
        * Goal: Allow the search to discard pages that can't be read, essentially ignoring errors. In my experimentation, some pages return errors despite everything being correct, so I don't want them to stop the whole search process
        * How: During the copy of pages, report the pages that returned errors and update the global pages list
        * Current workaround: insted of reporting a failure, I fill the buffer with zeros. This approach, however, wastes RAM and CPU power on useless searches, also having the potential to cause false matches when looking for 0.

    * [ ] QoL: Display a message to the user when the selected buffer is insufficient, instead of panicking
        * How: remove unwrap for the search and actually check for errors

* Modules
    * [x] Test moving closure API for thread pool
        * Attention: for some reason, the closure implementation is slightly slower due to dyn and Box 

* Platform support

    * [ ] Add linux support
        * Security: investigate how to work under [YAMA LSM](https://www.kernel.org/doc/html/v4.15/admin-guide/LSM/Yama.html) without disabling it
            * Giving root is not ideal
            * Giving debug caps is essentially root 
            * Other LSMs might help (AppArmor or SELinux), landlock forbids ptrace
            * Similar project might help: https://github.com/scanmem/scanmem and https://github.com/ugtrain/ugtrain
            * Be a parent?

            * Alternative: run as root, but setup a seccomp filter that only allows to trace a specific PID
                * Linux might have problems as PID are reused for different processes

            * Alternative: BPF LSM
                * Setup a rule that checks ptrace target name (process name)

* Known bugs

    * [ ] Small buffers cause an out of bounds access
        * Tests to reproduce: Use a non-contiguous victim process and as for a buffer smaller than the total sum of the non-contiguous segments
