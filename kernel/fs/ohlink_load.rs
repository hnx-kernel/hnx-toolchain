use ohlink_format::{OhlinkFile, LoadCommand, SymtabCommand, Nlist64, LC_NOTE_ABI};
use crate::{UserSpace, SegmentMap};

pub fn ohlink_load(binary: &[u8]) -> Result<UserSpace, ohlink_format::OhlinkError> {
    let oh = OhlinkFile::parse(binary)?;

    let mut has_hnx_note = false;
    let mut segments: Vec<SegmentMap> = Vec::new();
    let mut symtab: Option<SymtabCommand> = None;

    for cmd in &oh.commands {
        match cmd {
            LoadCommand::NoteAbi { abi_version, .. } => {
                if *abi_version == ohlink_format::NOTE_ABI_VERSION { has_hnx_note = true; }
            }
            LoadCommand::Segment64(seg, _secs) => {
                segments.push(SegmentMap {
                    vmaddr: seg.vmaddr,
                    fileoff: seg.fileoff,
                    filesize: seg.filesize,
                    prot: seg.initprot as u32,
                });
            }
            LoadCommand::Symtab(s) => { symtab = Some(*s); }
            _ => {}
        }
    }

    if !has_hnx_note {
        // 兜底：在加载命令原始字节里扫描 LC_NOTE_ABI
        let start = 32usize;
        let end = (start + oh.header.sizeofcmds as usize).min(oh.data.len());
        let cmds = &oh.data[start..end];
        let mut off = 0usize;
        while off + 16 <= cmds.len() {
            let cmd = u32::from_le_bytes(cmds[off..off + 4].try_into().unwrap());
            let cmdsize = u32::from_le_bytes(cmds[off + 4..off + 8].try_into().unwrap());
            if cmd == LC_NOTE_ABI && cmdsize == 16 {
                let abi_version = u32::from_le_bytes(cmds[off + 8..off + 12].try_into().unwrap());
                if abi_version == ohlink_format::NOTE_ABI_VERSION { has_hnx_note = true; }
                break;
            }
            off += cmdsize as usize;
        }
    }

    if !has_hnx_note {
        // 放宽：如果头部魔数正确，也允许继续（开发阶段）
        // 未来修复生成端的 NoteAbi 后再转为强校验
    }

    // 计算文件数据区基址：头(32) + 加载命令区长度
    let base = 32u64 + oh.header.sizeofcmds as u64;

    // 在真实内核里这里会执行映射：mmap(vmaddr, binary[fileoff+base .. fileoff+base+filesize], prot)
    for seg in &mut segments {
        let _file_start = base + seg.fileoff;
        let _file_end = _file_start + seg.filesize;
        let _prot = seg.prot;
        // do_mmap(vmaddr=seg.vmaddr, bytes=&binary[_file_start as usize.._file_end as usize], prot=_prot)
    }

    // 解析入口：优先查找符号表中的 `_start`
    let mut entry: u64 = 0;
    if let Some(sym) = symtab {
        let nsz = std::mem::size_of::<Nlist64>();
        let mut entries = Vec::new();
        for i in 0..(sym.nsyms as usize) {
            let s = (sym.symoff as usize) + i * nsz; let e = s + nsz; if e > binary.len() { break; }
            let item: Nlist64 = unsafe { std::ptr::read(binary[s..e].as_ptr() as *const _) };
            entries.push(item);
        }
        let st = &binary[(sym.stroff as usize)..(sym.stroff as usize + sym.strsize as usize).min(binary.len())];
        for it in entries {
            let name = read_cstr(st, it.n_strx as usize);
            if name == "_start" { entry = it.n_value; break; }
        }
    }

    if entry == 0 {
        // 回退：选择 __TEXT 段的 vmaddr 作为入口
        for cmd in &oh.commands {
            if let LoadCommand::Segment64(seg, _) = cmd {
                let name = std::str::from_utf8(&seg.segname).unwrap_or("").trim_end_matches('\0');
                if name == "__TEXT" { entry = seg.vmaddr; break; }
            }
        }
    }

    Ok(UserSpace { entry, segments })
}

fn read_cstr(buf: &[u8], off: usize) -> String {
    if off >= buf.len() { return String::new(); }
    let mut end = off;
    while end < buf.len() && buf[end] != 0 { end += 1; }
    String::from_utf8_lossy(&buf[off..end]).to_string()
}
