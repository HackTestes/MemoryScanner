cmake_minimum_required(VERSION 3.3)

project(MemoryScanner)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "./bin")

file(GLOB SRC_FILES src/*.cpp)
add_executable(MemoryScanner ${SRC_FILES})