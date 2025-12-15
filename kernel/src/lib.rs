pub mod fs;

#[derive(Debug, Clone)]
pub struct SegmentMap {
    pub vmaddr: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub prot: u32,
}

#[derive(Debug, Clone)]
pub struct UserSpace {
    pub entry: u64,
    pub segments: Vec<SegmentMap>,
}

