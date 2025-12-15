use anyhow::Result;
use clap::Parser;
use ohlink_format::{OhlinkFile, LoadCommand};

#[derive(Parser)]
#[command(author, version, about = "Display Ohlink file structure", long_about = None)]
struct Args {
    file: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data = std::fs::read(&args.file)?;
    let oh = OhlinkFile::parse(&data)?;
    println!("Magic   : {:02x?}", oh.header.magic);
    println!("CPU     : {:#x}", oh.header.cpu_type);
    println!("Type    : {:#x}", oh.header.file_type);
    println!("NCmds   : {}", oh.header.ncmds);
    for cmd in &oh.commands {
        match cmd {
            LoadCommand::Segment64(seg, secs) => {
                let name = std::str::from_utf8(&seg.segname).unwrap_or("").trim_end_matches('\0');
                println!("Segment {:8} vm={:#012x} fileoff={:#012x} filesz={:#x}", name, seg.vmaddr, seg.fileoff, seg.filesize);
                for s in secs {
                    let sname = std::str::from_utf8(&s.sectname).unwrap_or("").trim_end_matches('\0');
                    println!("  Section {:16} addr={:#012x} size={:#x}", sname, s.addr, s.size);
                }
            }
            LoadCommand::Symtab(sym) => {
                println!("Symtab  symoff={:#x} nsyms={} stroff={:#x}", sym.symoff, sym.nsyms, sym.stroff);
            }
            LoadCommand::NoteAbi { abi_version, flags } => {
                println!("NoteAbi version={} flags={:#x}", abi_version, flags);
            }
            _ => {}
        }
    }
    // 如果未打印 NoteAbi，额外扫描加载命令区进行兜底识别
    if !oh.commands.iter().any(|c| matches!(c, LoadCommand::NoteAbi { .. })) {
        let start = 32usize;
        let end = (start + oh.header.sizeofcmds as usize).min(oh.data.len());
        let cmds = &oh.data[start..end];
        let mut off = 0usize;
        while off + 16 <= cmds.len() {
            let cmd = u32::from_le_bytes(cmds[off..off + 4].try_into().unwrap());
            let cmdsize = u32::from_le_bytes(cmds[off + 4..off + 8].try_into().unwrap());
            if cmd == ohlink_format::LC_NOTE_ABI && cmdsize == 16 {
                let abi_version = u32::from_le_bytes(cmds[off + 8..off + 12].try_into().unwrap());
                let flags = u32::from_le_bytes(cmds[off + 12..off + 16].try_into().unwrap());
                println!("NoteAbi version={} flags={:#x}", abi_version, flags);
                break;
            }
            off += cmdsize as usize;
        }
    }
    Ok(())
}
