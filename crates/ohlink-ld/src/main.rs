use anyhow::{Context, Result};
use clap::Parser;
use ohlink_format::*;
use object::{Object, ObjectSection, ObjectSymbol};
use std::fs;
use std::path::PathBuf;
use std::mem::size_of;

/// 生成 FreeBSD 64 位风格四段布局
fn default_bsd_layout(_args: &Args) -> OhlinkBuilder {
    let mut b = OhlinkBuilder::new(MH_EXECUTE);
    b.add_segment("__PAGEZERO", 0x0)
        .add_section_with("__pagezero", &[], 0x0, 0x1000, 0x1_0000_0000);
    b
}
#[derive(Parser, Debug)]
#[command(author, version, about = "Link Ohlink object files into executable", long_about = None)]
struct Args {
    /// Input Ohlink object files
    inputs: Vec<PathBuf>,

    /// Output file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// TEXT segment base address
    #[arg(long, default_value_t = 0x4000_0000)]
    text_base: u64,

    /// DATA segment base address
    #[arg(long, default_value_t = 0x4000_8000)]
    data_base: u64,

    /// Entry symbol name
    #[arg(short = 'e', long)]
    entry: Option<String>,

    /// Build a library (.ohlib) archive instead of an executable
    #[arg(long, default_value_t = false)]
    library: bool,

    /// Include all members from any .ohlib inputs (no selective resolution)
    #[arg(long, default_value_t = false)]
    whole_archive: bool,
}

fn main() -> Result<()> {
    // 0. 捕获原始 argv，并解析常见 ld 开关（至少支持 -o 输出路径）
    let raw_args: Vec<String> = std::env::args().collect();
    let mut override_out: Option<PathBuf> = None;
    let mut filtered: Vec<String> = Vec::new();
    let mut i = 0;
    while i < raw_args.len() {
        let a = &raw_args[i];
        if a == "-o" && i + 1 < raw_args.len() {
            override_out = Some(PathBuf::from(raw_args[i + 1].clone()));
            i += 2;
            continue;
        }
        // 忽略其它以 '-' 开头的未知参数
        if a.starts_with('-') {
            i += 1;
            continue;
        }
        filtered.push(a.clone());
        i += 1;
    }

    // 1. 用过滤后的列表重新解析 CLI
    let args = Args::parse_from(filtered);
    
    if args.inputs.is_empty() { anyhow::bail!("no input files"); }

    let mut inputs_data: Vec<(PathBuf, Vec<u8>, OhlinkFile)> = Vec::new();
    let mut libraries: Vec<(PathBuf, OhlibArchive)> = Vec::new();

    for p in &args.inputs {
        // 0. 跳过目录
        if !p.is_file() {
            eprintln!("Skip non-file: {:?}", p);
            continue;
        }
        let d = fs::read(p).with_context(|| format!("Failed to read file: {:?}", p))?;
        if d.len() < 4 {
            eprintln!("Skip too-small file: {:?}", p);
            continue;
        }
        let magic: [u8; 4] = d[0..4].try_into().unwrap();

        // 1. 分流
        if magic == OHLIB_MAGIC {
            let arch = OhlibArchive::parse(&d)
                .with_context(|| format!("Failed to parse Ohlib: {:?}", p))?;
            libraries.push((p.clone(), arch));
        } else if magic == OHLINK_MAGIC || magic == OHLINK_MAGIC_64 {
            let file = OhlinkFile::parse(&d)
                .with_context(|| format!("Failed to parse Ohlink file: {:?}", p))?;
            inputs_data.push((p.clone(), d, file));
        } else {
            match object::File::parse(&*d) {
                Ok(elf) => {
                    let bytes = convert_elf_to_ohlink(&elf)?;
                    let file = OhlinkFile::parse(&bytes)?;
                    inputs_data.push((p.clone(), bytes, file));
                }
                Err(_) => {
                    eprintln!("Skip unknown format: {:?} (magic {:02x?})", p, magic);
                    let empty = OhlinkFile {
                        header: OhlinkHeader {
                            magic: OHLINK_MAGIC_64,
                            cpu_type: CPU_TYPE_ARM64,
                            cpu_subtype: 0,
                            file_type: MH_OBJECT,
                            ncmds: 0,
                            sizeofcmds: 0,
                            flags: 0,
                            reserved: 0,
                        },
                        commands: Vec::new(),
                        data: Vec::new(),
                    };
                    inputs_data.push((p.clone(), Vec::new(), empty));
                    continue;
                }
            }
        }
    }
    // Expand libraries: either whole-archive, or selective member inclusion by unresolved symbols
    if !libraries.is_empty() && !args.library {
        if args.whole_archive || inputs_data.is_empty() {
            // Include all members when requested, or when no base objects were provided
            for (lp, arch) in &libraries {
                for e in &arch.entries {
                    let name = String::from_utf8_lossy(&e.name).trim_end_matches('\0').to_string();
                    let start = e.offset as usize;
                    let end = start + e.size as usize;
                    if end > arch.data.len() { anyhow::bail!("ohlib member out of bounds: {:?}:{}", lp, name); }
                    let bytes = arch.data[start..end].to_vec();
                    let file = OhlinkFile::parse(&bytes).with_context(|| format!("Failed to parse member {} in {:?}", name, lp))?;
                    let mut pseudo = lp.clone();
                    pseudo.set_file_name(format!("{}({})", lp.file_name().unwrap().to_string_lossy(), name));
                    inputs_data.push((pseudo, bytes, file));
                }
            }
        } else {
            use std::collections::{HashSet, HashMap};
            let mut defined: HashSet<String> = HashSet::new();
            let mut undefined: HashSet<String> = HashSet::new();
            // Seed from existing object inputs
            for (_p, d, f) in &inputs_data {
                let mut symtab: Option<SymtabCommand> = None;
                for cmd in &f.commands { if let LoadCommand::Symtab(s) = cmd { symtab = Some(*s); } }
        if let Some(sym) = symtab {
            let nsz = size_of::<Nlist64>();
            let mut entries = Vec::new();
            for i in 0..(sym.nsyms as usize) {
                let s = (sym.symoff as usize) + i * nsz; let e = s + nsz; if s >= d.len() || e > d.len() { break; }
                let item: Nlist64 = unsafe { std::ptr::read(d[s..e].as_ptr() as *const _) };
                entries.push(item);
            }
            let st = if (sym.stroff as usize) < d.len() {
                let s = sym.stroff as usize; let e = (s + sym.strsize as usize).min(d.len());
                d[s..e].to_vec()
            } else { Vec::new() };
            for it in entries {
                let name = read_cstr(&st, it.n_strx as usize);
                if it.n_sect != 0 { defined.insert(name); } else { undefined.insert(name); }
            }
        }
            }
            if let Some(entry) = &args.entry { if !defined.contains(entry) { undefined.insert(entry.clone()); } }

            // Prepare candidates from libraries
            struct Candidate { name: String, path: PathBuf, bytes: Vec<u8>, file: OhlinkFile, defs: HashSet<String>, undefs: HashSet<String> }
            let mut candidates: Vec<Candidate> = Vec::new();
            for (lp, arch) in &libraries {
                for e in &arch.entries {
                    let mname = String::from_utf8_lossy(&e.name).trim_end_matches('\0').to_string();
                    let start = e.offset as usize; let end = start + e.size as usize; if end > arch.data.len() { continue; }
                    let bytes = arch.data[start..end].to_vec();
                    let file = match OhlinkFile::parse(&bytes) { Ok(f) => f, Err(_) => continue };
                    let mut defs = HashSet::new();
                    let mut undefs = HashSet::new();
                    let mut symtab: Option<SymtabCommand> = None;
                    for cmd in &file.commands { if let LoadCommand::Symtab(s) = cmd { symtab = Some(*s); } }
                    if let Some(sym) = symtab {
                        let nsz = size_of::<Nlist64>();
                        let mut entries = Vec::new();
                        for i in 0..(sym.nsyms as usize) {
                            let s = (sym.symoff as usize) + i * nsz; let e = s + nsz; if s >= bytes.len() || e > bytes.len() { break; }
                            let item: Nlist64 = unsafe { std::ptr::read(bytes[s..e].as_ptr() as *const _) };
                            entries.push(item);
                        }
                        let st = if (sym.stroff as usize) < bytes.len() {
                            let s = sym.stroff as usize; let e = (s + sym.strsize as usize).min(bytes.len());
                            bytes[s..e].to_vec()
                        } else { Vec::new() };
                        for it in entries { let nm = read_cstr(&st, it.n_strx as usize); if it.n_sect != 0 { defs.insert(nm); } else { undefs.insert(nm); } }
                    }
                    let mut pseudo = lp.clone(); pseudo.set_file_name(format!("{}({})", lp.file_name().unwrap().to_string_lossy(), mname));
                    candidates.push(Candidate { name: mname, path: pseudo, bytes, file, defs, undefs });
                }
            }

            let mut progress = true;
            while progress {
                progress = false;
                let mut i = 0;
                while i < candidates.len() {
                    let hit = !candidates[i].defs.is_disjoint(&undefined);
                    if hit {
                        // select this candidate
                        let cand = candidates.remove(i);
                        for nm in &cand.defs { undefined.remove(nm); defined.insert(nm.clone()); }
                        for nm in &cand.undefs { if !defined.contains(nm) { undefined.insert(nm.clone()); } }
                        inputs_data.push((cand.path, cand.bytes, cand.file));
                        progress = true;
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }

    // Library mode: package input objects into a .ohlib archive
    if args.library {
        let mut lib = OhlibBuilder::new();
        for (p, d, f) in &inputs_data {
            if f.header.file_type != MH_OBJECT {
                anyhow::bail!("only MH_OBJECT can be archived into .ohlib: {:?}", p);
            }
            let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "member".to_string());
            lib.add_member(&name, d);
        }
        let bytes = lib.build();
        let out = args.output.clone().unwrap_or_else(|| {
            let mut p = args.inputs[0].clone();
            p.set_extension("ohlib");
            p
        });
        fs::write(&out, &bytes).with_context(|| format!("Failed to write output: {:?}", out))?;
        println!("Archived: {} inputs -> {:?} ({} bytes)", args.inputs.len(), out, bytes.len());
        return Ok(());
    }

    let mut b = default_bsd_layout(&args);

    let mut text_items: Vec<(String, Vec<u8>, u32, u64, usize, u8, Section64)> = Vec::new();
    let mut data_items: Vec<(String, Vec<u8>, u32, u64, usize, u8, Section64)> = Vec::new();
    let mut sec_map: Vec<(usize, u8, u64)> = Vec::new(); // (file_idx, old_section_index, new_abs_base)
    let mut ord_map: Vec<(usize, u8, u8)> = Vec::new();  // (file_idx, old_section_index, new_ord)
    let mut text_off: u64 = 0;
    let mut data_off: u64 = 0;

    // 预解析所有输入的符号表
    let mut all_symbols: Vec<(usize, Vec<Nlist64>, Vec<u8>)> = Vec::new();
    for (fi, (_p, d, f)) in inputs_data.iter().enumerate() {
        let mut symtab: Option<SymtabCommand> = None;
        for cmd in &f.commands { if let LoadCommand::Symtab(s) = cmd { symtab = Some(*s); } }
        if let Some(sym) = symtab {
            let nlist_sz = size_of::<Nlist64>();
            let mut entries = Vec::new();
            for i in 0..(sym.nsyms as usize) {
                let start = (sym.symoff as usize) + i * nlist_sz;
                let end = start + nlist_sz;
                if start >= d.len() || end > d.len() { break; }
                let e: Nlist64 = unsafe { std::ptr::read(d[start..end].as_ptr() as *const _) };
                entries.push(e);
            }
            let st = if (sym.stroff as usize) < d.len() {
                let s = sym.stroff as usize;
                let e = (s + sym.strsize as usize).min(d.len());
                d[s..e].to_vec()
            } else { Vec::new() };
            all_symbols.push((fi, entries, st));
        } else {
            all_symbols.push((fi, Vec::new(), Vec::new()));
        }
    }

    // 合并节并应用重定位（生成待添加项）
    for (fi, (_p, d, f)) in inputs_data.iter().enumerate() {
        let mut old_sec_index: u8 = 0;
        for cmd in &f.commands {
            if let LoadCommand::Segment64(_seg, secs) = cmd {
                for sec in secs {
                    let segname = String::from_utf8_lossy(&sec.segname).trim_end_matches('\0').to_string();
                    let name = String::from_utf8_lossy(&sec.sectname).trim_end_matches('\0').to_string();
                    let mut data_slice = if sec.offset != 0 && sec.size > 0 {
                        let start = sec.offset as usize;
                        let end = start + sec.size as usize;
                        if start >= d.len() { Vec::new() } else {
                            let end = end.min(d.len());
                            if end <= start { Vec::new() } else { d[start..end].to_vec() }
                        }
                    } else { Vec::new() };

                    let (base_vmaddr, cur_off, is_data) = if segname == "__DATA" { (args.data_base, &mut data_off, true) } else { (args.text_base, &mut text_off, false) };
                    let align = sec.align as u64;
                    if align > 0 { *cur_off = align_up(*cur_off, align); }
                    let new_rel = *cur_off;
                    let new_abs = base_vmaddr + new_rel;

                    // 应用重定位：使用旧节地址计算偏移，使用新地址作为 place
                    if sec.nreloc > 0 {
                        apply_relocations_with_base(&mut data_slice, sec, new_abs, d, &all_symbols[fi].1)?;
                    }

                    if is_data {
                        data_items.push((name, data_slice, sec.align, new_rel, fi, old_sec_index, *sec));
                    } else {
                        text_items.push((name, data_slice, sec.align, new_rel, fi, old_sec_index, *sec));
                    }
                    sec_map.push((fi, old_sec_index, new_abs));
                    *cur_off += sec.size;
                    old_sec_index = old_sec_index.wrapping_add(1);
                }
            }
        }
    }

    // 添加段与节，生成 ord 映射
    {
        let text_seg = b.add_segment("__TEXT", args.text_base);
        for (name, data_slice, align, rel, fi, si, _old) in &text_items {
            text_seg.add_section_with(name, data_slice, *rel, *align, data_slice.len() as u64);
            let ord = ord_map.len() as u8;
            ord_map.push((*fi, *si, ord));
        }
    }
    {
        let data_seg = b.add_segment("__DATA", args.data_base);
        for (name, data_slice, align, rel, fi, si, _old) in &data_items {
            data_seg.add_section_with(name, data_slice, *rel, *align, data_slice.len() as u64);
            let ord = ord_map.len() as u8;
            ord_map.push((*fi, *si, ord));
        }
    }

    // 全局符号解析与重建符号表
    // 建立名称到地址映射以解析未定义符号
    let mut global_defs: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for (fi, entries, st) in &all_symbols {
        // 先记录定义符号的地址
        for e in entries {
            let name = read_cstr(st, e.n_strx as usize);
            if e.n_sect != 0 {
                // 找到该符号所在节的新基址
                let old_si = e.n_sect.saturating_sub(1);
                if let Some((_, _, base)) = sec_map.iter().find(|(f, s, _)| *f == *fi && *s as u8 == old_si).cloned() {
                    // 计算符号相对旧节的偏移
                    let old_sec = text_items.iter().chain(data_items.iter()).find(|(_, _, _, _, f, s, _)| *f == *fi && *s == old_si).map(|(_, _, _, _, _, _, sec)| *sec);
                    if let Some(sec_hdr) = old_sec {
                        let offset = (e.n_value as i128 - sec_hdr.addr as i128) as i128;
                        let new_val = (base as i128 + offset) as u64;
                        global_defs.insert(name.clone(), new_val);
                    }
                }
            }
        }
    }

    // 将所有符号写入输出符号表（未定义符号若可解析则赋值，否则报错）
    for (fi, entries, st) in &all_symbols {
        for e in entries {
            let name = read_cstr(st, e.n_strx as usize);
            let (new_val, sect_ord) = if e.n_sect != 0 {
                let old_si = e.n_sect.saturating_sub(1);
                if let Some((_, _, base)) = sec_map.iter().find(|(f, s, _)| *f == *fi && *s as u8 == old_si).cloned() {
                    let old_sec = text_items.iter().chain(data_items.iter()).find(|(_, _, _, _, f, s, _)| *f == *fi && *s == old_si).map(|(_, _, _, _, _, _, sec)| *sec).unwrap();
                    let offset = (e.n_value as i128 - old_sec.addr as i128) as i128;
                    let val = (base as i128 + offset) as u64;
                    let ord = ord_map.iter().find(|(f, s, _)| *f == *fi && *s as u8 == old_si).map(|(_, _, o)| *o).unwrap_or(0);
                    (val, ord)
                } else { (0, 0) }
            } else {
                // 未定义：尝试用 global_defs 解析
                let val = *global_defs.get(&name).unwrap_or(&0);
                (val, 0)
            };
            b.add_symbol_with(&name, new_val, sect_ord, e.n_type, e.n_desc);
        }
    }
    // 默认入口
    let entry_sym = args.entry.unwrap_or_else(|| "_start".to_string());
    let entry_val = *global_defs.get(&entry_sym).unwrap_or(&0);
    println!("Entry {} at {:#x}", entry_sym, entry_val);

    let bytes = b.build();
    let out = override_out
        .or(args.output.clone())
        .unwrap_or_else(|| {
            let mut p = args.inputs[0].clone();
            p.set_extension("exe.ohlink");
            p
        });
    fs::write(&out, &bytes).with_context(|| format!("Failed to write output: {:?}", out))?;
    println!("Linked: {} inputs -> {:?} ({} bytes)", args.inputs.len(), out, bytes.len());
    Ok(())
}

fn read_cstr(buf: &[u8], off: usize) -> String {
    if off >= buf.len() { return String::new(); }
    let mut end = off;
    while end < buf.len() && buf[end] != 0 { end += 1; }
    String::from_utf8_lossy(&buf[off..end]).to_string()
}

fn convert_elf_to_ohlink(elf: &object::File) -> Result<Vec<u8>> {
    use std::collections::HashMap;
    let mut builder = OhlinkBuilder::new(MH_OBJECT);

    let mut text_additions: Vec<(&'static str, Vec<u8>, u64, usize)> = Vec::new();
    let mut data_additions: Vec<(&'static str, Vec<u8>, u64, usize)> = Vec::new();

    for (elf_section_idx, section) in elf.sections().enumerate() {
        if let Ok(name) = section.name() {
            let is_text   = name.starts_with(".text");
            let is_rodata = name.starts_with(".rodata");
            let is_data   = name.starts_with(".data");
            let is_bss    = name.starts_with(".bss");
            if is_text {
                if let Ok(data) = section.data() { if !data.is_empty() { text_additions.push(("__text", data.to_vec(), section.address(), elf_section_idx)); } }
            } else if is_rodata {
                if let Ok(data) = section.data() { if !data.is_empty() { text_additions.push(("__rodata", data.to_vec(), section.address(), elf_section_idx)); } }
            } else if is_data {
                if let Ok(data) = section.data() { if !data.is_empty() { data_additions.push(("__data", data.to_vec(), section.address(), elf_section_idx)); } }
            } else if is_bss {
                if section.size() > 0 { data_additions.push(("__bss", Vec::new(), section.address(), elf_section_idx)); }
            }
        }
    }

    let mut section_map: HashMap<usize, u8> = HashMap::new();
    let mut section_ord: u8 = 0;
    {
        let text_segment = builder.add_segment("__TEXT", 0);
        for (name, data, addr, elf_idx) in text_additions.drain(..) {
            let align = elf.sections().nth(elf_idx).map(|s| s.align() as u32).unwrap_or(4);
            let size = elf.sections().nth(elf_idx).map(|s| s.size()).unwrap_or(data.len() as u64);
            text_segment.add_section_with(name, &data, addr, align, size);
            section_map.insert(elf_idx, section_ord);
            section_ord = section_ord.wrapping_add(1);
        }
    }
    {
        let data_segment = builder.add_segment("__DATA", 0);
        for (name, data, addr, elf_idx) in data_additions.drain(..) {
            let align = elf.sections().nth(elf_idx).map(|s| s.align() as u32).unwrap_or(4);
            let size = elf.sections().nth(elf_idx).map(|s| s.size()).unwrap_or(data.len() as u64);
            data_segment.add_section_with(name, &data, addr, align, size);
            section_map.insert(elf_idx, section_ord);
            section_ord = section_ord.wrapping_add(1);
        }
    }

    let mut elf_to_oh_sym: HashMap<usize, u32> = HashMap::new();
    for symbol in elf.symbols() {
        if let Ok(name) = symbol.name() {
            if name.is_empty() { continue; }
            let symbol_section = match symbol.section() {
                object::SymbolSection::Section(idx) => *section_map.get(&idx.0).unwrap_or(&0u8),
                _ => 0u8,
            };
            let n_type = if matches!(symbol.section(), object::SymbolSection::Undefined) { 0x00 } else if symbol.is_global() { 0x0f } else { 0x0e };
            let symbol_idx = builder.add_symbol_with(name, symbol.address(), symbol_section, n_type, 0);
            let elf_sym_idx = symbol.index().0;
            elf_to_oh_sym.insert(elf_sym_idx, symbol_idx);
        }
    }

    use std::collections::HashMap as Map;
    let mut reloc_map: Map<usize, Vec<Relocation64>> = Map::new();
    for (elf_section_idx, section) in elf.sections().enumerate() {
        for (offset, reloc) in section.relocations() {
            let r_addr = section.address().wrapping_add(offset);
            let r_symbol_elf = match reloc.target() { object::RelocationTarget::Symbol(si) => si.0, _ => 0 };
            let r_symbol = elf_to_oh_sym.get(&r_symbol_elf).copied().unwrap_or(0);
            let r_type = map_relocation_type(&reloc);
            let r_addend = reloc.addend();
            let r = Relocation64 { r_addr, r_symbol, r_type, r_addend };
            reloc_map.entry(elf_section_idx).or_default().push(r);
        }
    }
    for (elf_idx, relocs) in reloc_map.iter() {
        if let Some(ord) = section_map.get(elf_idx) { builder.add_relocations_by_ord(*ord, relocs); }
    }

    Ok(builder.build())
}

fn map_relocation_type(r: &object::Relocation) -> u32 {
    use object::RelocationKind as K;
    match r.kind() {
        K::Absolute => RELOC_ABS64,
        K::Relative => RELOC_REL64,
        K::PltRelative => RELOC_REL64,
        _ => RELOC_ABS64,
    }
}

fn apply_relocations_with_base(section_data: &mut [u8], old_sec: &Section64, new_abs_base: u64, file_data: &[u8], symbols: &[Nlist64]) -> Result<()> {
    let rs = old_sec.reloff as usize;
    let rsz = size_of::<Relocation64>();
    for i in 0..(old_sec.nreloc as usize) {
        let start = rs + i * rsz;
        let end = start + rsz;
        if end > file_data.len() { break; }
        let r: Relocation64 = unsafe { std::ptr::read(file_data[start..end].as_ptr() as *const _) };
        let offset_in_section = (r.r_addr - old_sec.addr) as usize;
        let place = (new_abs_base as i128) + (offset_in_section as i128);
        if offset_in_section + 8 > section_data.len() { continue; }

        let sym_idx = r.r_symbol as usize;
        if sym_idx >= symbols.len() { continue; }
        let sym = symbols[sym_idx];
        let target = sym.n_value as i128;
        let addend = r.r_addend as i128;

        match r.r_type {
            RELOC_ABS64 => {
                let val = (target + addend) as u64;
                section_data[offset_in_section..offset_in_section + 8].copy_from_slice(&val.to_le_bytes());
            }
            RELOC_ABS32 => {
                let val = target + addend;
                let v32 = val as i64;
                let lo = v32 as i32;
                section_data[offset_in_section..offset_in_section + 4].copy_from_slice(&lo.to_le_bytes());
            }
            RELOC_REL64 => {
                let delta = (target + addend) - place;
                let v = delta as i64;
                section_data[offset_in_section..offset_in_section + 8].copy_from_slice(&v.to_le_bytes());
            }
            RELOC_REL32 => {
                let delta = (target + addend) - place;
                let v = delta as i32;
                section_data[offset_in_section..offset_in_section + 4].copy_from_slice(&v.to_le_bytes());
            }
            RELOC_BRANCH26 => {
                // AArch64 B/BL: imm26 is ((target - place) >> 2), fits in signed 26 bits
                let delta = (target + addend) - place;
                let imm26 = (delta >> 2) as i32;
                let mask = 0x03ff_ffffu32; // 26 bits
                let orig = u32::from_le_bytes(section_data[offset_in_section..offset_in_section + 4].try_into().unwrap());
                let patched = (orig & !mask) | ((imm26 as u32) & mask);
                section_data[offset_in_section..offset_in_section + 4].copy_from_slice(&patched.to_le_bytes());
            }
            RELOC_AARCH64_ADR_PREL_PG_HI21 => {
                // Patch ADRP-style page-relative immediate: imm21 split into immlo[30:29] and immhi[23:5]
                // imm = sign21((page(target) - page(place)))
                let place_page = (place as i128) >> 12;
                let target_page = ((target + addend) as i128) >> 12;
                let imm = (target_page - place_page) as i32; // signed 21-bit
                let immlo = (imm & 0x3) as u32;         // bits[1:0]
                let immhi = ((imm >> 2) & 0x7ffff) as u32; // bits[20:2]
                let mut insn = u32::from_le_bytes(section_data[offset_in_section..offset_in_section + 4].try_into().unwrap());
                // Clear immlo[30:29], immhi[23:5]
                insn &= !(0b11 << 29);
                insn &= !(0x7ffff << 5);
                // Set new bits
                insn |= immlo << 29;
                insn |= immhi << 5;
                section_data[offset_in_section..offset_in_section + 4].copy_from_slice(&insn.to_le_bytes());
            }
            RELOC_AARCH64_ADD_ABS_LO12_NC => {
                // Patch ADD (immediate) imm12 in bits [21:10] with low12(target + addend)
                let lo12 = (((target + addend) as i64) & 0xfff) as u32;
                let mut insn = u32::from_le_bytes(section_data[offset_in_section..offset_in_section + 4].try_into().unwrap());
                insn &= !(0xfff << 10);
                insn |= lo12 << 10;
                section_data[offset_in_section..offset_in_section + 4].copy_from_slice(&insn.to_le_bytes());
            }
            RELOC_AARCH64_LD_PREL_LO19 => {
                // Patch LDR literal imm19 in bits [23:5] with ((target - place) >> 2)
                let delta = (target + addend) - place;
                let imm19 = (delta >> 2) as i32;
                let mut insn = u32::from_le_bytes(section_data[offset_in_section..offset_in_section + 4].try_into().unwrap());
                insn &= !(0x7ffff << 5);
                insn |= ((imm19 as u32) & 0x7ffff) << 5;
                section_data[offset_in_section..offset_in_section + 4].copy_from_slice(&insn.to_le_bytes());
            }
            _ => {
                // 复杂类型暂不应用，保留原值
            }
        }
    }
    Ok(())
}

fn align_up(x: u64, a: u64) -> u64 { if a == 0 { x } else { ((x + a - 1) / a) * a } }
