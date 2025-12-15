// crates/elf2ohlink/src/main.rs
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use ohlink_format::*;
use object::{Object, ObjectSection, ObjectSymbol};
use object::RelocationKind;
use object::elf;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input ELF file
    input: PathBuf,
    
    /// Output Ohlink file
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Output file type
    #[arg(long, value_enum, default_value_t = FileType::Object)]
    file_type: FileType,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum FileType {
    Object,
    Execute,
    Dylib,
}

impl From<FileType> for u32 {
    fn from(ft: FileType) -> Self {
        match ft {
            FileType::Object => MH_OBJECT,
            FileType::Execute => MH_EXECUTE,
            FileType::Dylib => MH_DYLIB,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // 1. 读取输入文件
    let data = fs::read(&args.input)
        .with_context(|| format!("Failed to read input file: {:?}", args.input))?;
    
    // 2. 解析 ELF 文件
    let obj = object::File::parse(&*data)
        .with_context(|| "Failed to parse ELF file")?;
    
    println!("=== ELF to Ohlink Converter ===");
    println!("Input: {:?}", args.input);
    println!("ELF type: {:?}", obj.kind());
    println!("Sections: {}", obj.sections().count());
    println!("Symbols: {}", obj.symbols().count());
    
    // 3. 转换为 Ohlink
    let ohlink_data = convert_elf_to_ohlink(&obj, args.file_type.into(), args.verbose)
        .with_context(|| "Conversion failed")?;
    
    // 4. 写入输出文件
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        path.set_extension("ohlink");
        path
    });
    
    fs::write(&output_path, &ohlink_data)
        .with_context(|| format!("Failed to write output: {:?}", output_path))?;
    
    // 5. 解析并显示结果
    let ohlink_file = OhlinkFile::parse(&ohlink_data)
        .with_context(|| "Failed to parse generated Ohlink file")?;
    
    println!("\n=== Conversion Results ===");
    println!("Output: {:?}", output_path);
    println!("Size: {} bytes", ohlink_data.len());
    let magic_le = u32::from_le_bytes(ohlink_file.header.magic);
    println!("Magic: {:#010x}", magic_le);
    println!("CPU: ARM64");
    println!("File type: {:#x}", ohlink_file.header.file_type);
    println!("Load commands: {}", ohlink_file.header.ncmds);
    
    // 显示段信息
    for (i, cmd) in ohlink_file.commands.iter().enumerate() {
        match cmd {
            LoadCommand::Segment64(segment, sections) => {
                let segname_cow = String::from_utf8_lossy(&segment.segname);
                let segname = segname_cow.trim_end_matches('\0');
                println!("\nSegment {}: {}", i, segname);
                println!("  VM range: {:#x} - {:#x}", 
                         segment.vmaddr, segment.vmaddr + segment.vmsize);
                println!("  File range: {:#x} - {:#x}", 
                         segment.fileoff, segment.fileoff + segment.filesize);
                println!("  Sections: {}", sections.len());
                
                for (j, section) in sections.iter().enumerate() {
                    let sectname_cow = String::from_utf8_lossy(&section.sectname);
                    let sectname = sectname_cow.trim_end_matches('\0');
                    println!("    [{:2}] {:16} addr:{:#010x} size:{:#6x} offset:{:#x}", 
                             j, sectname, section.addr, section.size, section.offset);
                }
            }
            LoadCommand::Symtab(symtab) => {
                println!("\nSymbol Table:");
                println!("  Symbols: {}", symtab.nsyms);
                println!("  Symbol offset: {:#x}", symtab.symoff);
                println!("  String table offset: {:#x}", symtab.stroff);
            }
            LoadCommand::Unknown { cmd, cmdsize, .. } => {
                println!("\nUnknown command: {:#x} (size: {})", cmd, cmdsize);
            }
            _ => { }
        }
    }
    
    println!("\n✅ Conversion successful!");
    Ok(())
}

fn convert_elf_to_ohlink(elf: &object::File, file_type: u32, verbose: bool) -> Result<Vec<u8>> {
    let mut builder = OhlinkBuilder::new(file_type);
    
    let mut text_additions: Vec<(&'static str, Vec<u8>, u64, usize)> = Vec::new();
    let mut data_additions: Vec<(&'static str, Vec<u8>, u64, usize)> = Vec::new();
    let mut symbol_mapping = Vec::new();
    
    for (elf_section_idx, section) in elf.sections().enumerate() {
        if let Ok(name) = section.name() {
            if verbose {
                println!("Processing section {}: '{}'", elf_section_idx, name);
            }
            
            match name {
                ".text" => {
                    if let Ok(data) = section.data() {
                        if !data.is_empty() {
                            let align = section.align() as u32;
                            let size = section.size();
                            text_additions.push(("__text", data.to_vec(), section.address(), elf_section_idx));
                            if verbose {
                                println!("  -> __TEXT,__text ({} bytes)", data.len());
                            }
                        }
                    }
                }
                ".data" => {
                    if let Ok(data) = section.data() {
                        if !data.is_empty() {
                            let align = section.align() as u32;
                            let size = section.size();
                            data_additions.push(("__data", data.to_vec(), section.address(), elf_section_idx));
                            if verbose {
                                println!("  -> __DATA,__data ({} bytes)", data.len());
                            }
                        }
                    }
                }
                ".rodata" => {
                    if let Ok(data) = section.data() {
                        if !data.is_empty() {
                            let align = section.align() as u32;
                            let size = section.size();
                            text_additions.push(("__cstring", data.to_vec(), section.address(), elf_section_idx));
                            if verbose {
                                println!("  -> __TEXT,__cstring ({} bytes)", data.len());
                            }
                        }
                    }
                }
                name if name.starts_with(".rodata.str") => {
                    if let Ok(data) = section.data() {
                        if !data.is_empty() {
                            let align = section.align() as u32;
                            let size = section.size();
                            text_additions.push(("__cstring", data.to_vec(), section.address(), elf_section_idx));
                            if verbose {
                                println!("  -> __TEXT,__cstring ({} bytes)", data.len());
                            }
                        }
                    }
                }
                ".bss" => {
                    if section.size() > 0 {
                        let align = section.align() as u32;
                        let size = section.size();
                        data_additions.push(("__bss", Vec::new(), section.address(), elf_section_idx));
                        if verbose {
                            println!("  -> __DATA,__bss ({} bytes, zero-filled)", section.size());
                        }
                    }
                }
                _ => {
                    if verbose && !name.starts_with('.') {
                        println!("  Skipping non-standard section: {}", name);
                    }
                }
            }
        }
    }
    
    // 先构建节与段，建立 ELF->Ohlink 节索引映射后再转换符号
    
    let mut section_map: HashMap<usize, u8> = HashMap::new();
    let mut section_ord: u8 = 0;
    {
        let text_segment = builder.add_segment("__TEXT", 0x4000_0000);
        for (name, data, addr, elf_idx) in text_additions.drain(..) {
            let align = elf.sections().nth(elf_idx).map(|s| s.align() as u32).unwrap_or(4);
            let size = elf.sections().nth(elf_idx).map(|s| s.size()).unwrap_or(data.len() as u64);
            text_segment.add_section_with(name, &data, addr, align, size);
            section_map.insert(elf_idx, section_ord);
            section_ord = section_ord.wrapping_add(1);
        }
    }
    {
        let data_segment = builder.add_segment("__DATA", 0x4000_8000);
        for (name, data, addr, elf_idx) in data_additions.drain(..) {
            let align = elf.sections().nth(elf_idx).map(|s| s.align() as u32).unwrap_or(4);
            let size = elf.sections().nth(elf_idx).map(|s| s.size()).unwrap_or(data.len() as u64);
            data_segment.add_section_with(name, &data, addr, align, size);
            section_map.insert(elf_idx, section_ord);
            section_ord = section_ord.wrapping_add(1);
        }
    }

    // 转换符号（现在已有节索引映射）
    let mut elf_to_oh_sym: HashMap<usize, u32> = HashMap::new();
    for symbol in elf.symbols() {
        if let Ok(name) = symbol.name() {
            if name.is_empty() {
                continue;
            }

            let symbol_section = match symbol.section() {
                object::SymbolSection::Section(idx) => *section_map.get(&idx.0).unwrap_or(&0u8),
                _ => 0u8,
            };

            if verbose && symbol.kind() == object::SymbolKind::Text {
                println!(
                    "Adding symbol: {} at {:#x} (section: {})",
                    name, symbol.address(), symbol_section
                );
            }

            let n_type = if matches!(symbol.section(), object::SymbolSection::Undefined)
                { 0x00 } else if symbol.is_global() { 0x0f } else { 0x0e };
            let symbol_idx = builder.add_symbol_with(name, symbol.address(), symbol_section, n_type, 0);
            // 建立 ELF 符号索引到 Ohlink 符号索引的映射
            let elf_sym_idx = symbol.index().0;
            elf_to_oh_sym.insert(elf_sym_idx, symbol_idx);
            symbol_mapping.push((name.to_string(), symbol_idx));
        }
    }

    if verbose {
        println!("\nSymbol mapping:");
        for (name, idx) in &symbol_mapping {
            println!("  {} -> symbol index {}", name, idx);
        }
    }

    // 收集并写入重定位信息
    let mut reloc_map: HashMap<usize, Vec<Relocation64>> = HashMap::new();
    for (elf_section_idx, section) in elf.sections().enumerate() {
        for (offset, reloc) in section.relocations() {
            let r_addr = section.address().wrapping_add(offset);
            let r_symbol_elf = match reloc.target() {
                object::RelocationTarget::Symbol(si) => si.0,
                _ => 0,
            };
            let r_symbol = elf_to_oh_sym.get(&r_symbol_elf).copied().unwrap_or(0);
            let r_type = map_relocation_type(&reloc);
            let r_addend = reloc.addend();
            let r = Relocation64 { r_addr, r_symbol, r_type, r_addend };
            reloc_map.entry(elf_section_idx).or_default().push(r);
        }
    }
    // 写入到相应的 Ohlink 节（按 ord）
    for (elf_idx, relocs) in reloc_map.iter() {
        if let Some(ord) = section_map.get(elf_idx) {
            builder.add_relocations_by_ord(*ord, relocs);
        }
    }

    let segments_count = builder.segment_count();
    let symbols_count = builder.symbol_count();
    let ohlink_data = builder.build();
    
    if verbose {
        println!("\nGenerated Ohlink file:");
        println!("  Total size: {} bytes", ohlink_data.len());
        println!("  Segments: {}", segments_count);
        println!("  Symbols: {}", symbols_count);
    }
    
    Ok(ohlink_data)
}

fn map_relocation_type(reloc: &object::Relocation) -> u32 {
    match reloc.kind() {
        RelocationKind::Absolute => match reloc.size() {
            64 => RELOC_ABS64,
            32 => RELOC_ABS32,
            _ => RELOC_NONE,
        },
        RelocationKind::Relative => match reloc.size() {
            64 => RELOC_REL64,
            32 => RELOC_REL32,
            _ => RELOC_NONE,
        },
        RelocationKind::Got | RelocationKind::GotRelative | RelocationKind::GotBaseRelative | RelocationKind::GotBaseOffset => RELOC_GOT,
        RelocationKind::PltRelative => RELOC_PLT,
        RelocationKind::Elf(t) => match t {
            elf::R_AARCH64_CALL26 | elf::R_AARCH64_JUMP26 => RELOC_BRANCH26,
            elf::R_AARCH64_ADR_PREL_PG_HI21 => RELOC_AARCH64_ADR_PREL_PG_HI21,
            elf::R_AARCH64_ADD_ABS_LO12_NC => RELOC_AARCH64_ADD_ABS_LO12_NC,
            elf::R_AARCH64_LD_PREL_LO19 => RELOC_AARCH64_LD_PREL_LO19,
            _ => RELOC_NONE,
        },
        _ => RELOC_NONE,
    }
}
