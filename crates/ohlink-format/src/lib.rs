// crates/ohlink-format/src/lib.rs
#![allow(non_camel_case_types)]

use thiserror::Error;
// ====== 3. 顶部加工具函数 ======
#[inline]
fn align_up(val: u64, align: u64) -> u64 {
    if align == 0 { val } else { ((val + align - 1) / align) * align }
}
// ==================== 错误类型 ====================
#[derive(Error, Debug)]
pub enum OhlinkError {
    #[error("Invalid magic number: expected {expected:?}, found {found:?}")]
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },
    #[error("Unsupported CPU type: {0:#x}")]
    UnsupportedCpuType(u32),
    #[error("Unsupported file type: {0:#x}")]
    UnsupportedFileType(u32),
    #[error("Parse error at offset {offset:#x}: {message}")]
    ParseError { offset: u64, message: String },
}

pub type Result<T> = std::result::Result<T, OhlinkError>;

// ==================== 常量定义 ====================
pub const OHLINK_MAGIC: [u8; 4] = 0x0f112233u32.to_le_bytes();
pub const OHLINK_MAGIC_64: [u8; 4] = 0x0f112234u32.to_le_bytes();
pub const OHLIB_MAGIC: [u8; 4] = 0x0f112235u32.to_le_bytes();
pub const CPU_TYPE_ARM64: u32 = 0x0100_000C;
pub const MH_OBJECT: u32 = 0x1;
pub const MH_EXECUTE: u32 = 0x2;
pub const MH_DYLIB: u32 = 0x6;
pub const LC_SEGMENT_64: u32 = 0x19;
pub const LC_SYMTAB: u32 = 0x2;
pub const RELOC_NONE: u32 = 0;
pub const RELOC_ABS64: u32 = 1;
pub const RELOC_ABS32: u32 = 2;
pub const RELOC_REL64: u32 = 3;
pub const RELOC_REL32: u32 = 4;
pub const RELOC_BRANCH26: u32 = 5;
pub const RELOC_GOT: u32 = 6;
pub const RELOC_PLT: u32 = 7;
pub const RELOC_TLS: u32 = 8;
pub const RELOC_AARCH64_ADR_PREL_PG_HI21: u32 = 9;
pub const RELOC_AARCH64_ADD_ABS_LO12_NC: u32 = 10;
pub const RELOC_AARCH64_LD_PREL_LO19: u32 = 11;
pub const LC_NOTE_ABI: u32 = 0x31;
pub const NOTE_NAME_HNX: &[u8; 4] = b"HNX\0";
pub const NOTE_ABI_VERSION: u32 = 1;
// ==================== 核心结构 ====================
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OhlinkHeader {
    pub magic: [u8; 4],
    pub cpu_type: u32,
    pub cpu_subtype: u32,
    pub file_type: u32,
    pub ncmds: u32,
    pub sizeofcmds: u32,
    pub flags: u32,
    pub reserved: u32,
}

impl OhlinkHeader {
    pub fn is_64bit(&self) -> bool {
        self.magic == OHLINK_MAGIC_64
    }

    pub fn validate(&self) -> Result<()> {
        if self.magic != OHLINK_MAGIC && self.magic != OHLINK_MAGIC_64 {
            return Err(OhlinkError::InvalidMagic {
                expected: OHLINK_MAGIC,
                found: self.magic,
            });
        }

        if self.cpu_type != CPU_TYPE_ARM64 {
            return Err(OhlinkError::UnsupportedCpuType(self.cpu_type));
        }

        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&self.magic);
        bytes.extend_from_slice(&self.cpu_type.to_le_bytes());
        bytes.extend_from_slice(&self.cpu_subtype.to_le_bytes());
        bytes.extend_from_slice(&self.file_type.to_le_bytes());
        bytes.extend_from_slice(&self.ncmds.to_le_bytes());
        bytes.extend_from_slice(&self.sizeofcmds.to_le_bytes());
        bytes.extend_from_slice(&self.flags.to_le_bytes());
        bytes.extend_from_slice(&self.reserved.to_le_bytes());
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 32 {
            return Err(OhlinkError::ParseError {
                offset: 0,
                message: "Data too short for Ohlink header".to_string(),
            });
        }

        let magic: [u8; 4] = data[0..4].try_into().unwrap();
        let cpu_type = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let cpu_subtype = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let file_type = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let ncmds = u32::from_le_bytes(data[16..20].try_into().unwrap());
        let sizeofcmds = u32::from_le_bytes(data[20..24].try_into().unwrap());
        let flags = u32::from_le_bytes(data[24..28].try_into().unwrap());
        let reserved = u32::from_le_bytes(data[28..32].try_into().unwrap());

        Ok(Self {
            magic,
            cpu_type,
            cpu_subtype,
            file_type,
            ncmds,
            sizeofcmds,
            flags,
            reserved,
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SegmentCommand64 {
    pub cmd: u32,
    pub cmdsize: u32,
    pub segname: [u8; 16],
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub maxprot: i32,
    pub initprot: i32,
    pub nsects: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Section64 {
    pub sectname: [u8; 16],
    pub segname: [u8; 16],
    pub addr: u64,
    pub size: u64,
    pub offset: u32,
    pub align: u32,
    pub reloff: u32,
    pub nreloc: u32,
    pub flags: u32,
    pub reserved1: u32,
    pub reserved2: u32,
    pub reserved3: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SymtabCommand {
    pub cmd: u32,
    pub cmdsize: u32,
    pub symoff: u32,
    pub nsyms: u32,
    pub stroff: u32,
    pub strsize: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Relocation64 {
    pub r_addr: u64,
    pub r_symbol: u32,
    pub r_type: u32,
    pub r_addend: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Nlist64 {
    pub n_strx: u32,
    pub n_type: u8,
    pub n_sect: u8,
    pub n_desc: u16,
    pub n_value: u64,
}

// ==================== 文件结构 ====================
#[derive(Debug, Clone)]
pub enum LoadCommand {
    Segment64(SegmentCommand64, Vec<Section64>),
    Symtab(SymtabCommand),
    Unknown {
        cmd: u32,
        cmdsize: u32,
        data: Vec<u8>,
    },
    NoteAbi { abi_version: u32, flags: u32 },
}

#[derive(Debug)]
pub struct OhlinkFile {
    pub header: OhlinkHeader,
    pub commands: Vec<LoadCommand>,
    pub data: Vec<u8>,
}

impl OhlinkFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let header = OhlinkHeader::from_bytes(&data[0..32])?;
        header.validate()?;

        let mut commands = Vec::new();
        let mut offset = 32;

        for _ in 0..header.ncmds {
            if offset + 8 > data.len() {
                return Err(OhlinkError::ParseError {
                    offset: offset as u64,
                    message: "Incomplete load command".to_string(),
                });
            }

            let cmd = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let cmdsize = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());

            match cmd {
                LC_SEGMENT_64 => {
                    if cmdsize < 72 {
                        return Err(OhlinkError::ParseError {
                            offset: offset as u64,
                            message: format!("Segment command too small: {}", cmdsize),
                        });
                    }

                    let segment_cmd: SegmentCommand64 =
                        unsafe { std::ptr::read(data[offset..offset + 72].as_ptr() as *const _) };

                    let nsects = segment_cmd.nsects as usize;
                    let mut sections = Vec::with_capacity(nsects);

                    let mut section_offset = offset + 72;
                    for _ in 0..nsects {
                        if section_offset + 80 > data.len() {
                            return Err(OhlinkError::ParseError {
                                offset: section_offset as u64,
                                message: "Incomplete section".to_string(),
                            });
                        }

                        let section: Section64 = unsafe {
                            std::ptr::read(
                                data[section_offset..section_offset + 80].as_ptr() as *const _
                            )
                        };
                        sections.push(section);
                        section_offset += 80;
                    }

                    commands.push(LoadCommand::Segment64(segment_cmd, sections));
                }
                LC_SYMTAB => {
                    if cmdsize != 24 {
                        return Err(OhlinkError::ParseError {
                            offset: offset as u64,
                            message: format!("Invalid symtab command size: {}", cmdsize),
                        });
                    }

                    let symtab_cmd: SymtabCommand =
                        unsafe { std::ptr::read(data[offset..offset + 24].as_ptr() as *const _) };
                    commands.push(LoadCommand::Symtab(symtab_cmd));
                }
                LC_NOTE_ABI => {
                    if cmdsize != 16 {
                        return Err(OhlinkError::ParseError {
                            offset: offset as u64,
                            message: format!("Invalid NoteAbi size: {}", cmdsize),
                        });
                    }
                    let abi_version = u32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());
                    let flags = u32::from_le_bytes(data[offset + 12..offset + 16].try_into().unwrap());
                    commands.push(LoadCommand::NoteAbi { abi_version, flags });
                }
                _ => {
                    let end = (offset + cmdsize as usize).min(data.len());
                    let cmd_data = data[offset..end].to_vec();
                    commands.push(LoadCommand::Unknown {
                        cmd,
                        cmdsize,
                        data: cmd_data,
                    });
                }
            }

            offset += cmdsize as usize;
        }

        Ok(Self {
            header,
            commands,
            data: data.to_vec(),
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OhlibHeader {
    pub magic: [u8; 4],
    pub nentries: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OhlibEntry {
    pub name: [u8; 32],
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug)]
pub struct OhlibArchive {
    pub header: OhlibHeader,
    pub entries: Vec<OhlibEntry>,
    pub data: Vec<u8>,
}

impl OhlibArchive {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < std::mem::size_of::<OhlibHeader>() { return Err(OhlinkError::ParseError { offset: 0, message: "Data too short for Ohlib header".to_string() }); }
        let header: OhlibHeader = unsafe { std::ptr::read(data[0..std::mem::size_of::<OhlibHeader>()].as_ptr() as *const _) };
        if header.magic != OHLIB_MAGIC { return Err(OhlinkError::InvalidMagic { expected: OHLIB_MAGIC, found: header.magic }); }
        let mut entries = Vec::with_capacity(header.nentries as usize);
        let mut off = std::mem::size_of::<OhlibHeader>();
        for _ in 0..header.nentries {
            if off + std::mem::size_of::<OhlibEntry>() > data.len() { return Err(OhlinkError::ParseError { offset: off as u64, message: "Incomplete ohlib entry".to_string() }); }
            let e: OhlibEntry = unsafe { std::ptr::read(data[off..off + std::mem::size_of::<OhlibEntry>()].as_ptr() as *const _) };
            entries.push(e);
            off += std::mem::size_of::<OhlibEntry>();
        }
        Ok(Self { header, entries, data: data.to_vec() })
    }
}

pub struct OhlibBuilder {
    entries: Vec<(String, Vec<u8>)>,
}

impl OhlibBuilder {
    pub fn new() -> Self { Self { entries: Vec::new() } }
    pub fn add_member(&mut self, name: &str, bytes: &[u8]) { self.entries.push((name.to_string(), bytes.to_vec())); }
    pub fn build(self) -> Vec<u8> {
        let n = self.entries.len();
        let hsz = std::mem::size_of::<OhlibHeader>();
        let esz = std::mem::size_of::<OhlibEntry>();
        let header = OhlibHeader { magic: OHLIB_MAGIC, nentries: n as u32, reserved: 0 };
        let mut result = Vec::new();
        result.resize(hsz + n * esz, 0);
        let mut cursor = hsz;
        let mut data_off = (hsz + n * esz) as u64;
        let mut data_blob = Vec::new();
        for (name, bytes) in self.entries {
            let mut entry = OhlibEntry { name: [0; 32], offset: data_off, size: bytes.len() as u64 };
            let nb = name.as_bytes();
            let nl = nb.len().min(31);
            entry.name[..nl].copy_from_slice(&nb[..nl]);
            let ebytes = unsafe { std::slice::from_raw_parts(&entry as *const _ as *const u8, esz) };
            result[cursor..cursor + esz].copy_from_slice(ebytes);
            cursor += esz;
            data_blob.extend_from_slice(&bytes);
            data_off += bytes.len() as u64;
        }
        let hbytes = unsafe { std::slice::from_raw_parts(&header as *const _ as *const u8, hsz) };
        result[0..hsz].copy_from_slice(hbytes);
        result.extend_from_slice(&data_blob);
        result
    }
}

// ==================== 构建器 ====================
pub struct OhlinkBuilder {
    file_type: u32,
    segments: Vec<SegmentBuilder>,
    symbols: Vec<SymbolEntry>,
    strings: Vec<u8>,
}

impl OhlinkBuilder {
    pub fn new(file_type: u32) -> Self {
        Self {
            file_type,
            segments: Vec::new(),
            symbols: Vec::new(),
            strings: vec![0], // 字符串表以空字符开始
        }
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    pub fn add_segment(&mut self, name: &str, vmaddr: u64) -> &mut SegmentBuilder {
        let mut segname = [0; 16];
        let bytes = name.as_bytes();
        let len = bytes.len().min(15);
        segname[..len].copy_from_slice(&bytes[..len]);

        self.segments.push(SegmentBuilder {
            segname,
            vmaddr,
            maxprot: 7, // RWX
            initprot: 7,
            flags: 0,
            sections: Vec::new(),
        });

        self.segments.last_mut().unwrap()
    }

    pub fn add_relocations_by_ord(&mut self, ord: u8, relocs: &[Relocation64]) {
        let mut count: usize = 0;
        let target = ord as usize;
        for seg in &mut self.segments {
            for sec in &mut seg.sections {
                if count == target {
                    sec.relocations.extend_from_slice(relocs);
                    return;
                }
                count += 1;
            }
        }
    }

    pub fn add_symbol(&mut self, name: &str, value: u64, sect: u8) -> u32 {
        let n_strx = self.strings.len() as u32;
        self.strings.extend_from_slice(name.as_bytes());
        self.strings.push(0);

        let index = self.symbols.len() as u32;
        self.symbols.push(SymbolEntry {
            n_strx,
            n_type: 0x0f,     // N_SECT | N_EXT
            n_sect: sect + 1, // 段索引从1开始
            n_desc: 0,
            n_value: value,
        });

        index
    }

    pub fn add_symbol_with(&mut self, name: &str, value: u64, sect: u8, n_type: u8, n_desc: u16) -> u32 {
        let n_strx = self.strings.len() as u32;
        self.strings.extend_from_slice(name.as_bytes());
        self.strings.push(0);

        let index = self.symbols.len() as u32;
        self.symbols.push(SymbolEntry {
            n_strx,
            n_type,
            n_sect: sect + 1,
            n_desc,
            n_value: value,
        });

        index
    }

    pub fn build(mut self) -> Vec<u8> {
        let mut result = Vec::new();
        let mut load_commands = Vec::new();
        // HNX ABI note —— 必须存在
        // load_commands.push(LoadCommand::NoteAbi {
        //     abi_version: NOTE_ABI_VERSION,
        //     flags: 0,
        // });

        // 1. 预留头部空间
        result.resize(32, 0);
        let mut file_offset = 32u64;

        // 2. 计算加载命令总大小以确定数据区基址
        let sizeof_segment_cmd = std::mem::size_of::<SegmentCommand64>();
        let sizeof_section = std::mem::size_of::<Section64>();
        let sizeof_symtab_cmd = std::mem::size_of::<SymtabCommand>();
        let note_abi_size = 16; // cmd+u32 + cmdsize+u32 + abi_version+u32 + flags+u32
        let load_commands_size: usize = self
            .segments
            .iter()
            .map(|seg| sizeof_segment_cmd + seg.sections.len() * sizeof_section)
            .sum::<usize>()
            + sizeof_symtab_cmd
            + note_abi_size; // <-- 把 NoteAbi 算进来

        let base_offset = 32u64 + load_commands_size as u64;

        // 3. 构建段 - 使用 drain 来转移所有权，并修正偏移为绝对文件偏移
        let segments = std::mem::take(&mut self.segments); // 取走所有权
        let segment_count = segments.len();
        for segment in segments {
            let (mut segment_cmd, mut sections, section_data) = segment.build(&mut file_offset);

            // 修正偏移：加上命令区长度
            segment_cmd.fileoff = segment_cmd.fileoff + base_offset;
            for sec in &mut sections {
                sec.offset = (sec.offset as u64 + base_offset) as u32;
                if sec.reloff != 0 { sec.reloff = (sec.reloff as u64 + base_offset) as u32; }
            }

            // 序列化段命令
            let cmd_bytes = unsafe {
                std::slice::from_raw_parts(
                    &segment_cmd as *const _ as *const u8,
                    std::mem::size_of::<SegmentCommand64>(),
                )
            };
            load_commands.extend_from_slice(cmd_bytes);

            // 序列化区头
            for section in &sections {
                let section_bytes = unsafe {
                    std::slice::from_raw_parts(
                        section as *const _ as *const u8,
                        std::mem::size_of::<Section64>(),
                    )
                };
                load_commands.extend_from_slice(section_bytes);
            }

            result.extend_from_slice(&section_data);
        }

        // 4. 构建符号表
        let symtab_offset = file_offset as u32;
        let nsyms = self.symbols.len() as u32;

        for symbol in &self.symbols {
            let nlist = symbol.to_nlist64();
            let symbol_bytes = unsafe {
                std::slice::from_raw_parts(
                    &nlist as *const _ as *const u8,
                    std::mem::size_of::<Nlist64>(),
                )
            };
            result.extend_from_slice(symbol_bytes);
            file_offset += std::mem::size_of::<Nlist64>() as u64;
        }

        // 5. 构建字符串表
        let stroff = file_offset as u32;
        result.extend_from_slice(&self.strings);

        // 6. 将 Symtab 与 NoteAbi 命令写入加载命令区
        let mut symtab_cmd = SymtabCommand {
            cmd: LC_SYMTAB,
            cmdsize: std::mem::size_of::<SymtabCommand>() as u32,
            symoff: (symtab_offset as u64 + base_offset) as u32,
            nsyms,
            stroff: (stroff as u64 + base_offset) as u32,
            strsize: self.strings.len() as u32,
        };
        let sym_bytes = unsafe {
            std::slice::from_raw_parts(
                &symtab_cmd as *const _ as *const u8,
                std::mem::size_of::<SymtabCommand>(),
            )
        };
        load_commands.extend_from_slice(sym_bytes);
        load_commands.extend_from_slice(&LC_NOTE_ABI.to_le_bytes());
        load_commands.extend_from_slice(&16u32.to_le_bytes());
        load_commands.extend_from_slice(&NOTE_ABI_VERSION.to_le_bytes());
        load_commands.extend_from_slice(&0u32.to_le_bytes());
        // for cmd in &load_commands {
        //     match cmd {
        //         LoadCommand::Segment64(seg, secs) => {
        //             cmd_bytes.extend_from_slice(unsafe {
        //                 std::slice::from_raw_parts(seg as *const _ as *const u8,
        //                                         std::mem::size_of::<SegmentCommand64>())
        //             });
        //             for sec in secs {
        //                 cmd_bytes.extend_from_slice(unsafe {
        //                     std::slice::from_raw_parts(sec as *const _ as *const u8,
        //                                             std::mem::size_of::<Section64>())
        //                 });
        //             }
        //         }
        //         LoadCommand::Symtab(sym) => {
        //             cmd_bytes.extend_from_slice(unsafe {
        //                 std::slice::from_raw_parts(sym as *const _ as *const u8,
        //                                         std::mem::size_of::<SymtabCommand>())
        //             });
        //         }
        //         LoadCommand::NoteAbi { abi_version, flags } => {
        //             // 手写 NoteAbi 命令结构：cmd + cmdsize + abi_version + flags = 16 字节
        //             let cmd: u32 = LC_NOTE_ABI;
        //             let cmdsize: u32 = 16;
        //             cmd_bytes.extend_from_slice(&cmd.to_le_bytes());
        //             cmd_bytes.extend_from_slice(&cmdsize.to_le_bytes());
        //             cmd_bytes.extend_from_slice(&abi_version.to_le_bytes());
        //             cmd_bytes.extend_from_slice(&flags.to_le_bytes());
        //         }
        //         _ => {} // 其他 Unknown 命令不管
        //     }
        // }
        // 7. 构建头部
        let header = OhlinkHeader {
            magic: OHLINK_MAGIC_64,
            cpu_type: CPU_TYPE_ARM64,
            cpu_subtype: 0,
            file_type: self.file_type,
            ncmds: (segment_count + 2) as u32, // 段 + 符号表命令 + NoteAbi
            sizeofcmds: load_commands.len() as u32,
            flags: 0,
            reserved: 0,
        };

        // 8. 写入头部和加载命令
        let header_bytes = header.to_bytes();
        result[..32].copy_from_slice(&header_bytes);

        let mut final_result = Vec::new();
        final_result.extend_from_slice(&header_bytes);
        final_result.extend_from_slice(&load_commands);
        final_result.extend_from_slice(&result[32..]);

        final_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_parse_offsets() {
        let mut b = OhlinkBuilder::new(MH_OBJECT);
        {
            let text = b.add_segment("__TEXT", 0x4000_0000);
            text.add_section("__text", &[1, 2, 3, 4], 0x0);
        }
        b.add_symbol("_start", 0x4000_0000, 0);

        let bytes = b.build();
        let parsed = OhlinkFile::parse(&bytes).expect("parse");

        assert_eq!(parsed.header.magic, OHLINK_MAGIC_64);
        assert_eq!(parsed.header.ncmds, 2);

        let mut seg_opt: Option<(SegmentCommand64, Vec<Section64>)> = None;
        let mut sym_opt: Option<SymtabCommand> = None;
        for cmd in parsed.commands.iter() {
            match cmd {
                LoadCommand::Segment64(s, secs) => seg_opt = Some((*s, secs.clone())),
                LoadCommand::Symtab(sym) => sym_opt = Some(*sym),
                _ => {}
            }
        }

        let (seg, secs) = seg_opt.expect("segment");
        let sym = sym_opt.expect("symtab");

        // 数据区起始应为 32 + sizeofcmds
        let base = 32u64 + parsed.header.sizeofcmds as u64;
        assert!(seg.fileoff >= base);
        assert!(secs[0].offset as u64 >= base);

        // 符号表应紧随段数据之后
        let data_end = seg.fileoff + seg.filesize;
        assert_eq!(sym.symoff as u64, data_end);
        let nlist_size = std::mem::size_of::<Nlist64>() as u64;
        assert_eq!(sym.stroff as u64, sym.symoff as u64 + nlist_size);
    }
}

pub struct SegmentBuilder {
    segname: [u8; 16],
    vmaddr: u64,
    maxprot: i32,
    initprot: i32,
    flags: u32,
    sections: Vec<SectionBuilder>,
}

impl SegmentBuilder {
    pub fn add_section(&mut self, name: &str, data: &[u8], addr: u64) -> &mut Self {
        let mut sectname = [0; 16];
        let bytes = name.as_bytes();
        let len = bytes.len().min(15);
        sectname[..len].copy_from_slice(&bytes[..len]);

        self.sections.push(SectionBuilder {
            sectname,
            addr,
            size: data.len() as u64,
            data: data.to_vec(),
            align: 4,
            relocations: Vec::new(),
        });

        self
    }

    pub fn add_section_with(&mut self, name: &str, data: &[u8], addr: u64, align: u32, size: u64) -> &mut Self {
        let mut sectname = [0; 16];
        let bytes = name.as_bytes();
        let len = bytes.len().min(15);
        sectname[..len].copy_from_slice(&bytes[..len]);

        self.sections.push(SectionBuilder {
            sectname,
            addr,
            size,
            data: data.to_vec(),
            align,
            relocations: Vec::new(),
        });

        self
    }

    fn build(mut self, file_offset: &mut u64) -> (SegmentCommand64, Vec<Section64>, Vec<u8>) {
        let nsects = self.sections.len() as u32;
        let mut section_headers = Vec::new();
        let mut section_data = Vec::new();

        let mut vmend = self.vmaddr;
        let fileoff = *file_offset;

        // 使用 drain 来转移 sections 的所有权
        for section in self.sections.drain(..) {
            // 对齐
            let align = section.align as u64;
            if align > 0 {
                let remainder = *file_offset % align;
                if remainder != 0 {
                    let pad = align - remainder;
                    // 文件对齐需要填充实际字节
                    section_data.resize(section_data.len() + pad as usize, 0);
                    *file_offset += pad;
                }
            }

            let offset_field = if section.data.is_empty() { 0 } else { *file_offset as u32 };
            if !section.data.is_empty() {
                section_data.extend_from_slice(&section.data);
                *file_offset += section.data.len() as u64;
            }

            let mut reloff_field: u32 = 0;
            let mut nreloc_field: u32 = 0;
            if !section.relocations.is_empty() {
                reloff_field = *file_offset as u32;
                nreloc_field = section.relocations.len() as u32;
                for r in &section.relocations {
                    let r_bytes = unsafe {
                        std::slice::from_raw_parts(
                            r as *const _ as *const u8,
                            std::mem::size_of::<Relocation64>(),
                        )
                    };
                    section_data.extend_from_slice(r_bytes);
                    *file_offset += std::mem::size_of::<Relocation64>() as u64;
                }
            }

            let section_header = Section64 {
                sectname: section.sectname,
                segname: self.segname,
                addr: self.vmaddr + section.addr,
                size: section.size,
                offset: offset_field,
                align: section.align,
                reloff: reloff_field,
                nreloc: nreloc_field,
                flags: 0,
                reserved1: 0,
                reserved2: 0,
                reserved3: 0,
            };

            section_headers.push(section_header);
            vmend = vmend.max(self.vmaddr + section.addr + section.size);
        }

        let segment_cmd = SegmentCommand64 {
            cmd: LC_SEGMENT_64,
            cmdsize: (std::mem::size_of::<SegmentCommand64>()
                + nsects as usize * std::mem::size_of::<Section64>()) as u32,
            segname: self.segname,
            vmaddr: self.vmaddr,
            vmsize: vmend - self.vmaddr,
            fileoff,
            filesize: *file_offset - fileoff,
            maxprot: self.maxprot,
            initprot: self.initprot,
            nsects,
            flags: self.flags,
        };

        (segment_cmd, section_headers, section_data)
    }
}

struct SectionBuilder {
    sectname: [u8; 16],
    addr: u64,
    size: u64,
    data: Vec<u8>,
    align: u32,
    relocations: Vec<Relocation64>,
}

#[derive(Debug, Clone)]
struct SymbolEntry {
    n_strx: u32,
    n_type: u8,
    n_sect: u8,
    n_desc: u16,
    n_value: u64,
}

impl SymbolEntry {
    fn to_nlist64(&self) -> Nlist64 {
        Nlist64 {
            n_strx: self.n_strx,
            n_type: self.n_type,
            n_sect: self.n_sect,
            n_desc: self.n_desc,
            n_value: self.n_value,
        }
    }
}
