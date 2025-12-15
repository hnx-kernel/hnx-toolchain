use kernel::fs::ohlink_load::ohlink_load;

fn main() {
    let p = "/Users/admin/Desktop/personal/code/hnx-toolchain/target/aarch64-hnx-ohlink/debug/hello.ohlink";
    let data = std::fs::read(p).expect("read hello.ohlink");
    match ohlink_load(&data) {
        Ok(us) => {
            println!("Loaded entry={:#x} segments={}", us.entry, us.segments.len());
            for (i, s) in us.segments.iter().enumerate() {
                println!("  [{}] vmaddr={:#x} fileoff={:#x} size={:#x} prot={:#x}", i, s.vmaddr, s.fileoff, s.filesize, s.prot);
            }
        }
        Err(e) => {
            eprintln!("Load error: {:?}", e);
        }
    }
}
