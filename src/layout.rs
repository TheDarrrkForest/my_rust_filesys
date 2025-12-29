pub const BLOCK_SIZE: usize = 4096;
pub const INODE_SIZE: usize = 128;
pub const DIR_ENTRY_SIZE: usize = 64;
pub const MAGIC: u32 = 0x12345678;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum InodeType {
    Unused = 0,
    File = 1,
    Directory = 2,
}

pub struct Superblock {
    pub magic: u32,
    pub total_blocks: u32,
    pub inode_count: u32,
    pub data_area_start: u32,
}

impl Superblock {
    pub fn serialize(&self) -> [u8; BLOCK_SIZE] {
        let mut buf = [0u8; BLOCK_SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4..8].copy_from_slice(&self.total_blocks.to_le_bytes());
        buf[8..12].copy_from_slice(&self.inode_count.to_le_bytes());
        buf[12..16].copy_from_slice(&self.data_area_start.to_le_bytes());
        buf
    }
}

pub struct Inode {
    pub mode: InodeType,
    pub size: u32,
    pub blocks: [u32; 12],
}

impl Inode {
    pub fn serialize(&self) -> [u8; INODE_SIZE] {
        let mut buf = [0u8; INODE_SIZE];
        buf[0..4].copy_from_slice(&(self.mode as u32).to_le_bytes());
        buf[4..8].copy_from_slice(&self.size.to_le_bytes());
        for i in 0..12 {
            buf[8 + i * 4..12 + i * 4].copy_from_slice(&self.blocks[i].to_le_bytes());
        }
        buf
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        let mode_raw = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let mode = match mode_raw {
            1 => InodeType::File,
            2 => InodeType::Directory,
            _ => InodeType::Unused,
        };
        let size = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let mut blocks = [0u32; 12];
        for i in 0..12 {
            blocks[i] = u32::from_le_bytes(buf[8 + i * 4..12 + i * 4].try_into().unwrap());
        }
        Inode { mode, size, blocks }
    }
}

pub struct DirEntry {
    pub inode_no: u32,
    pub name: String,
}

impl DirEntry {
    pub fn serialize(&self) -> [u8; DIR_ENTRY_SIZE] {
        let mut buf = [0u8; DIR_ENTRY_SIZE];
        buf[0..4].copy_from_slice(&self.inode_no.to_le_bytes());
        let name_bytes = self.name.as_bytes();
        let len = name_bytes.len().min(60);
        buf[4..4 + len].copy_from_slice(&name_bytes[..len]);
        buf
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        let inode_no = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let name_end = buf[4..].iter().position(|&b| b == 0).unwrap_or(60);
        let name = String::from_utf8_lossy(&buf[4..4 + name_end]).into_owned();
        DirEntry { inode_no, name }
    }
}