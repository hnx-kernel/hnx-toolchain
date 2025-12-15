# Ohlink Binary Format Specification

## Overview
Ohlink is a custom binary format for the HNX kernel, inspired by Mach-O but simplified for embedded/kernel use.

## File Structure
1. **Header** (32/40 bytes)
2. **Load Commands** (variable size)
3. **Data** (sections, symbol tables, etc.)

## Header Format
| Offset | Size | Field       | Description                  |
|--------|------|-------------|------------------------------|
| 0x00   | 4    | magic       | "OHLK" (32-bit) or "OHL+" (64-bit) |
| 0x04   | 4    | cpu_type    | CPU architecture identifier  |
| 0x08   | 4    | cpu_subtype | CPU variant                  |
| 0x0C   | 4    | file_type   | Object, executable, dylib, etc. |
| 0x10   | 4    | ncmds       | Number of load commands      |
| 0x14   | 4    | sizeofcmds  | Total size of all commands   |
| 0x18   | 4    | flags       | Miscellaneous flags          |
| 0x1C   | 4    | reserved    | Reserved (64-bit only)       |

## Load Commands
Each load command has:
- cmd (4 bytes): Command type
- cmdsize (4 bytes): Total command size including sections

## Planned Commands
1. LC_SEGMENT_64 - Define a memory segment
2. LC_SYMTAB - Symbol table
3. LC_DYSYMTAB - Dynamic symbol table
4. LC_CODE_SIGNATURE - Code signing (future)

## Comparison with Mach-O
- Simplified segment/section model
- No fat binary support initially
- Custom relocation format optimized for aarch64
- Integrated debugging information
