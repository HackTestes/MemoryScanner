#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <psapi.h>
#include <iostream>
#include <string>

#include "MemoryRegion.hpp"


unsigned long show_module(MEMORY_BASIC_INFORMATION info) {
    unsigned long usage = 0;

    std::cout << info.BaseAddress << "(" << info.RegionSize / 1024 << "KiB)\t";

    switch (info.State)
    {
    case MEM_COMMIT:
        std::cout << "Committed";
        break;
    case MEM_RESERVE:
        std::cout << "Reserved";
        break;
    case MEM_FREE:
        std::cout << "Free";
        break;
    }

    std::cout << "\t";

    switch (info.Type)
    {
    case MEM_IMAGE:
        std::cout << "Code Module";
        break;
    case MEM_MAPPED:
        std::cout << "Mapped     ";
        break;
    case MEM_PRIVATE:
        std::cout << "Private    ";
    }

    std::cout << "\t";

    int guard = 0, nocache = 0;

    if ( info.AllocationProtect & PAGE_NOCACHE)
        nocache = 1;
    if ( info.AllocationProtect & PAGE_GUARD )
        guard = 1;

    info.AllocationProtect &= ~(PAGE_GUARD | PAGE_NOCACHE);

    if ((info.State == MEM_COMMIT) && (info.AllocationProtect == PAGE_READWRITE || info.AllocationProtect == PAGE_READONLY))
        usage += info.RegionSize;

    switch (info.AllocationProtect)
    {
    case PAGE_READONLY:
        std::cout << "Read Only";
        break;
    case PAGE_READWRITE:
        std::cout << "Read/Write";
        break;
    case PAGE_WRITECOPY:
        std::cout << "Copy on Write";
        break;
    case PAGE_EXECUTE:
        std::cout << "Execute only";
        break;
    case PAGE_EXECUTE_READ:
        std::cout << "Execute/Read";
        break;
    case PAGE_EXECUTE_READWRITE:
        std::cout << "Execute/Read/Write";
        break;
    case PAGE_EXECUTE_WRITECOPY:
        std::cout << "COW Executable";
        break;
    }

    if (guard)
        std::cout << "\tguard page";
    if (nocache)
        std::cout << "\tnon-cacheable";
    std::cout << "\n";

    return usage;
}

unsigned long read_memory_pages(HANDLE process)
{
    unsigned long usage = 0;

    unsigned char* p = NULL;
    MEMORY_BASIC_INFORMATION info;

    for ( p = NULL; VirtualQueryEx(process, p, &info, sizeof(info)) == sizeof(info); p += info.RegionSize )
    {
        if(info.AllocationProtect == PAGE_READWRITE | info.AllocationProtect == PAGE_EXECUTE_READWRITE)
            usage += show_module(info);
    }
    return usage;
}


int main(int argc, char **argv)
{
    // Opens a process based on its PID
    DWORD process_id = std::stoi(argv[1]);
    HANDLE process_handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE , false, process_id);

    if(process_handle == nullptr)
    {
        return EXIT_FAILURE;
    }

    unsigned long mem_used = read_memory_pages(process_handle);
    std::cout << "Total memory used: " << mem_used / 1024 << "KB\n";

    // Closes the process handle
    if(CloseHandle(process_handle) == 0)
    {
        return EXIT_FAILURE;
    }

    //---------------------------------------Tests--------------------------------------------------------------

    int foo = 15; // 0x00 00 00 0F
    int lol = 33;
    unsigned char* not_foo;

    unsigned char buffer[100]={0};

    memcpy(buffer, &foo, sizeof(int)); //either memmove
    memcpy(buffer+4, &lol, sizeof(int)); //either memmove

    /*for(int i=0; i<4; ++i)
    {
        std::cout << int(buffer[i]) << " "; 
    }*/

    std::cout << "\n" << std::endl;

    not_foo = buffer;
    int not_not_foo = int(*not_foo);
    std::cout << not_not_foo << "\n";

    //bool comparison = int(*not_foo) == 33;

    buffer[0] = 0;
    std::cout << not_not_foo << "\n";

    return EXIT_SUCCESS;
}