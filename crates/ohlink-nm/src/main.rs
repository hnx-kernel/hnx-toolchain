use anyhow::{Context, Result};
use clap::Parser;
use ohlink_format::*;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "List symbols from Ohlink file", long_about = None)]
struct Args {
    input: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data = fs::read(&args.input)
        .with_context(|| format!("Failed to read file: {:?}", args.input))?;
    let magic: [u8; 4] = data[0..4].try_into().unwrap();
    if magic == OHLIB_MAGIC {
        let arch = OhlibArchive::parse(&data).with_context(|| "Failed to parse Ohlib archive")?;
        for e in &arch.entries {
            let mname = String::from_utf8_lossy(&e.name).trim_end_matches('\0').to_string();
            let start = e.offset as usize; let end = start + e.size as usize; if end > arch.data.len() { continue; }
            let bytes = arch.data[start..end].to_vec();
            if let Ok(file) = OhlinkFile::parse(&bytes) {
                let mut symtab: Option<SymtabCommand> = None;
                for cmd in &file.commands { if let LoadCommand::Symtab(s) = cmd { symtab = Some(*s); } }
                if let Some(sym) = symtab {
                    let nsz = std::mem::size_of::<Nlist64>();
                    let mut entries: Vec<Nlist64> = Vec::new();
                    for i in 0..(sym.nsyms as usize) {
                        let s = (sym.symoff as usize) + i * nsz; let e = s + nsz; if e > bytes.len() { break; }
                        let item: Nlist64 = unsafe { std::ptr::read(bytes[s..e].as_ptr() as *const _) };
                        entries.push(item);
                    }
                    let strtab = &bytes[(sym.stroff as usize)..(sym.stroff as usize + sym.strsize as usize).min(bytes.len())];
                    for it in entries { let name = read_cstr(strtab, it.n_strx as usize); println!("{:#018x} {}({})", it.n_value, mname, name); }
                }
            }
        }
    } else {
        let file = OhlinkFile::parse(&data).with_context(|| "Failed to parse Ohlink file")?;
        let mut symtab: Option<SymtabCommand> = None;
        for cmd in &file.commands { if let LoadCommand::Symtab(s) = cmd { symtab = Some(*s); } }
        let sym = symtab.context("No symbol table")?;
        let nlist_sz = std::mem::size_of::<Nlist64>();
        let mut entries = Vec::new();
        for i in 0..(sym.nsyms as usize) {
            let start = (sym.symoff as usize) + i * nlist_sz;
            let end = start + nlist_sz;
            if end > data.len() { break; }
            let e: Nlist64 = unsafe { std::ptr::read(data[start..end].as_ptr() as *const _) };
            entries.push(e);
        }
        let strtab = &data[(sym.stroff as usize)..(sym.stroff as usize + sym.strsize as usize).min(data.len())];
        for e in entries { let name = read_cstr(strtab, e.n_strx as usize); println!("{:#018x} {}", e.n_value, name); }
    }
    Ok(())
}

fn read_cstr(buf: &[u8], off: usize) -> String {
    if off >= buf.len() { return String::new(); }
    let mut end = off;
    while end < buf.len() && buf[end] != 0 { end += 1; }
    String::from_utf8_lossy(&buf[off..end]).to_string()
}
