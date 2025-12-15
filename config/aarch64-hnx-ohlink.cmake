# CMake toolchain file for aarch64-hnx-ohlink
set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR aarch64)
set(CMAKE_SYSTEM_VERSION 1)

# Target triple
set(TARGET_TRIPLE "aarch64-hnx-ohlink")

# Compiler paths
set(CMAKE_C_COMPILER clang)
set(CMAKE_C_COMPILER_TARGET ${TARGET_TRIPLE})
set(CMAKE_CXX_COMPILER clang++)
set(CMAKE_CXX_COMPILER_TARGET ${TARGET_TRIPLE})
set(CMAKE_ASM_COMPILER clang)
set(CMAKE_ASM_COMPILER_TARGET ${TARGET_TRIPLE})

# Compiler flags
set(CMAKE_C_FLAGS_INIT 
    "-target ${TARGET_TRIPLE} \
     -ffreestanding \
     -fno-stack-protector \
     -fno-builtin \
     -mgeneral-regs-only \
     -mno-outline-atomics")
     
set(CMAKE_CXX_FLAGS_INIT 
    "${CMAKE_C_FLAGS_INIT} \
     -fno-exceptions \
     -fno-rtti \
     -nostdinc++")
     
set(CMAKE_ASM_FLAGS_INIT 
    "-target ${TARGET_TRIPLE} \
     -x assembler-with-cpp")

# Linker
set(CMAKE_LINKER "ld.lld")
set(CMAKE_EXE_LINKER_FLAGS_INIT
    "-target ${TARGET_TRIPLE} \
     -nostdlib \
     -static \
     -Wl,--gc-sections \
     -Wl,-z,max-page-size=4096")

# Search paths
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

# Binary format
set(CMAKE_EXECUTABLE_SUFFIX ".ohlink")
set(CMAKE_SHARED_LIBRARY_SUFFIX "")
set(CMAKE_STATIC_LIBRARY_SUFFIX ".a")

# Skip compiler tests
set(CMAKE_C_COMPILER_WORKS TRUE)
set(CMAKE_CXX_COMPILER_WORKS TRUE)