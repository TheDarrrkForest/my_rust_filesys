use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use crate::layout::BLOCK_SIZE;

pub struct BlockDevice {
    file: File,
}

impl BlockDevice {
    pub fn open(path: &str) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        BlockDevice { file }
    }

    pub fn read_block(&mut self, block_idx: u32, buf: &mut [u8; BLOCK_SIZE]) {
        self.file.seek(SeekFrom::Start(block_idx as u64 * BLOCK_SIZE as u64)).unwrap();
        self.file.read_exact(buf).unwrap();
    }

    pub fn write_block(&mut self, block_idx: u32, buf: &[u8; BLOCK_SIZE]) {
        self.file.seek(SeekFrom::Start(block_idx as u64 * BLOCK_SIZE as u64)).unwrap();
        self.file.write_all(buf).unwrap();
    }
}