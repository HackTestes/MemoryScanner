#include <windows.h>
#include "MemoryRegion.hpp"

// Minimal info to create this obj
MemoryRegion::MemoryRegion(unsigned char *BaseAddress_arg, SIZE_T RegionSize_arg)
{
    this->lpBaseAddress = BaseAddress_arg;
    this->RegionSize = RegionSize_arg;
    this->memory_buffer = new unsigned char[RegionSize_arg]; // Reserves memory based on RegionSize in bytes
    this->BytesTransferred = 0; // Verify is ReadProcessMemory read all of the bytes from this memory region
}

MemoryRegion::~MemoryRegion()
{
    delete []memory_buffer;
}

