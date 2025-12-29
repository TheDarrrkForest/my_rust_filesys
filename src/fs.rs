use crate::device::BlockDevice;
use crate::layout::*;
use std::io::Read;

pub struct MyFileSystem {
    device: BlockDevice,
    pub cwd_ino: u32,     // 当前目录的 Inode 编号
    pub cwd_path: String, // 当前路径的字符串表示
}

impl MyFileSystem {
    pub fn new(disk_path: &str) -> Self {
        MyFileSystem {
            device: BlockDevice::open(disk_path),
            cwd_ino: 1, // 初始指向根目录
            cwd_path: "/".to_string(),
        }
    }

    // 格式化
    pub fn format(&mut self) {
        println!("Formatting disk...");

        // 1. 写入超级块 (Block 0)
        let sb = Superblock {
            magic: MAGIC,
            total_blocks: 1024,
            inode_count: 128,
            data_area_start: 10,
        };
        self.device.write_block(0, &sb.serialize());

        // 2. 初始化数据块位图 (Block 2)
        let mut data_bitmap = [0u8; BLOCK_SIZE];

        data_bitmap[0] = 0xFF;
        data_bitmap[1] = 0b00000111;

        self.device.write_block(2, &data_bitmap);

        // 3. 初始化 Inode 位图 (Block 1)
        let mut inode_bitmap = [0u8; BLOCK_SIZE];
        inode_bitmap[0] = 0b00000011;
        self.device.write_block(1, &inode_bitmap);

        // 4. 初始化根目录 Inode (Inode 1) 存放在 Block 3
        let mut root_inode = Inode {
            mode: InodeType::Directory,
            size: BLOCK_SIZE as u32,
            blocks: [0; 12],
        };
        root_inode.blocks[0] = 10;

        let mut inode_table_block = [0u8; BLOCK_SIZE];
        inode_table_block[INODE_SIZE..INODE_SIZE * 2].copy_from_slice(&root_inode.serialize());
        self.device.write_block(3, &inode_table_block);

        // 5. 初始化根目录的数据块 (Block 10)
        let mut data_block = [0u8; BLOCK_SIZE];
        let dot = DirEntry {
            inode_no: 1,
            name: ".".to_string(),
        };
        let dotdot = DirEntry {
            inode_no: 1,
            name: "..".to_string(),
        };
        data_block[0..64].copy_from_slice(&dot.serialize());
        data_block[64..128].copy_from_slice(&dotdot.serialize());
        self.device.write_block(10, &data_block);

        // 6. 初始化当前目录
        self.cwd_ino = 1;
        self.cwd_path = "/".to_string();

        println!("Done.");
    }

    // ls 命令
    pub fn list_dir(&mut self, path: &str) {
        // 1. 获取路径对应的 Inode 编号
        let ino = match self.resolve_path(path) {
            Some(i) => i,
            None => {
                println!("ls: {}: No such file or directory", path);
                return;
            }
        };

        // 2. 读取该 Inode
        let inode = self.read_inode(ino);

        // 3. 检查是否为目录
        if inode.mode != InodeType::Directory {
            println!("{}", path.split('/').last().unwrap_or(path));
            return;
        }

        println!("Listing directory: {}", path);
        let mut buf = [0u8; BLOCK_SIZE];

        // 4. 遍历 Inode 指向的所有数据块
        for &data_block_idx in inode.blocks.iter() {
            if data_block_idx == 0 {
                continue;
            }

            self.device.read_block(data_block_idx, &mut buf);

            // 5. 在每个块中按 DIR_ENTRY_SIZE (64字节) 长度切分并解析
            for i in 0..(BLOCK_SIZE / DIR_ENTRY_SIZE) {
                let start = i * DIR_ENTRY_SIZE;
                let end = start + DIR_ENTRY_SIZE;

                let entry = DirEntry::deserialize(&buf[start..end]);

                if entry.inode_no != 0 {
                    let child_inode = self.read_inode(entry.inode_no);
                    let suffix = match child_inode.mode {
                        InodeType::Directory => "/",
                        _ => "",
                    };
                    println!("  {}{}", entry.name, suffix);
                }
            }
        }
    }

    fn read_inode(&mut self, ino: u32) -> Inode {
        let mut buf = [0u8; BLOCK_SIZE];
        let block_idx = 3 + (ino * INODE_SIZE as u32) / BLOCK_SIZE as u32;
        let offset_in_block = (ino * INODE_SIZE as u32) % BLOCK_SIZE as u32;

        self.device.read_block(block_idx, &mut buf);
        let start = offset_in_block as usize;
        Inode::deserialize(&buf[start..start + INODE_SIZE])
    }

    fn find_in_dir(&mut self, dir_ino: u32, name: &str) -> Option<u32> {
        let inode = self.read_inode(dir_ino);
        if inode.mode != InodeType::Directory {
            return None;
        }

        let mut buf = [0u8; BLOCK_SIZE];
        for &data_block_idx in inode.blocks.iter() {
            if data_block_idx == 0 {
                continue;
            }
            self.device.read_block(data_block_idx, &mut buf);

            for i in 0..(BLOCK_SIZE / DIR_ENTRY_SIZE) {
                let start = i * DIR_ENTRY_SIZE;
                let entry = DirEntry::deserialize(&buf[start..start + DIR_ENTRY_SIZE]);
                if entry.inode_no != 0 && entry.name == name {
                    return Some(entry.inode_no);
                }
            }
        }
        None
    }

    pub fn resolve_path(&mut self, path: &str) -> Option<u32> {
        if path == "/" {
            return Some(1);
        }

        let mut current_ino = 1;
        let parts = path.split('/').filter(|s| !s.is_empty());

        for part in parts {
            match self.find_in_dir(current_ino, part) {
                Some(next_ino) => current_ino = next_ino,
                None => return None,
            }
        }
        Some(current_ino)
    }

    // cd 命令
    pub fn cd(&mut self, abs_path: &str) {
        if let Some(ino) = self.resolve_path(abs_path) {
            let inode = self.read_inode(ino);
            if inode.mode == InodeType::Directory {
                self.cwd_ino = ino;
                self.cwd_path = abs_path.to_string();
            } else {
                println!("cd: not a directory: {}", abs_path);
            }
        } else {
            println!("cd: no such directory: {}", abs_path);
        }
    }

    fn allocate_bit(&mut self, bitmap_block_idx: u32) -> Option<u32> {
        let mut buf = [0u8; BLOCK_SIZE];
        self.device.read_block(bitmap_block_idx, &mut buf);

        for i in 0..BLOCK_SIZE {
            if buf[i] != 0xFF {
                for bit in 0..8 {
                    if (buf[i] & (1 << bit)) == 0 {
                        buf[i] |= 1 << bit;
                        self.device.write_block(bitmap_block_idx, &buf);
                        return Some((i * 8 + bit) as u32);
                    }
                }
            }
        }
        None
    }

    fn alloc_inode(&mut self) -> Option<u32> {
        self.allocate_bit(1)
    }

    fn alloc_data_block(&mut self) -> Option<u32> {
        self.allocate_bit(2)
    }

    // mkdir 命令
    pub fn mkdir(&mut self, path: &str) {
        // 1. 分离父路径和新目录名
        let (parent_path, dir_name) = match path.rsplit_once('/') {
            Some(("", name)) => ("/", name),
            Some((p, name)) => (p, name),
            None => {
                println!("mkdir: Invalid path");
                return;
            }
        };

        // 2. 找到父目录 Inode
        let parent_ino = match self.resolve_path(parent_path) {
            Some(ino) => ino,
            None => {
                println!("mkdir: Parent not found: {}", parent_path);
                return;
            }
        };

        // 3. 检查是否已存在
        if self.find_in_dir(parent_ino, dir_name).is_some() {
            println!("mkdir: '{}' already exists", dir_name);
            return;
        }

        // 4. 分配资源
        let new_ino = self.alloc_inode().expect("No free Inodes");
        let new_data_block = self.alloc_data_block().expect("No free Data Blocks");

        // 5. 初始化并写入新 Inode
        let new_inode = Inode {
            mode: InodeType::Directory,
            size: BLOCK_SIZE as u32,
            blocks: {
                let mut b = [0u32; 12];
                b[0] = new_data_block;
                b
            },
        };
        let mut table_buf = [0u8; BLOCK_SIZE];
        let block_idx = 3 + (new_ino * INODE_SIZE as u32) / BLOCK_SIZE as u32;
        let offset = (new_ino * INODE_SIZE as u32) % BLOCK_SIZE as u32;
        self.device.read_block(block_idx, &mut table_buf);
        table_buf[offset as usize..offset as usize + INODE_SIZE]
            .copy_from_slice(&new_inode.serialize());
        self.device.write_block(block_idx, &table_buf);

        // 6. 初始化新目录的数据块
        let mut new_data = [0u8; BLOCK_SIZE];
        let dot = DirEntry {
            inode_no: new_ino,
            name: ".".to_string(),
        };
        let dotdot = DirEntry {
            inode_no: parent_ino,
            name: "..".to_string(),
        };
        new_data[0..64].copy_from_slice(&dot.serialize());
        new_data[64..128].copy_from_slice(&dotdot.serialize());
        self.device.write_block(new_data_block, &new_data);

        // 7. 在父目录中增加条目
        self.add_dir_entry(parent_ino, new_ino, dir_name);

        println!("Directory '{}' created.", dir_name);
    }

    fn add_dir_entry(&mut self, parent_ino: u32, child_ino: u32, name: &str) {
        let mut parent_inode = self.read_inode(parent_ino);
        let mut buf = [0u8; BLOCK_SIZE];

        let block_idx = parent_inode.blocks[0];
        self.device.read_block(block_idx, &mut buf);

        for i in 0..(BLOCK_SIZE / DIR_ENTRY_SIZE) {
            let start = i * DIR_ENTRY_SIZE;
            let entry = DirEntry::deserialize(&buf[start..start + DIR_ENTRY_SIZE]);
            if entry.inode_no == 0 {
                let new_entry = DirEntry {
                    inode_no: child_ino,
                    name: name.to_string(),
                };
                buf[start..start + DIR_ENTRY_SIZE].copy_from_slice(&new_entry.serialize());
                self.device.write_block(block_idx, &buf);
                return;
            }
        }
        panic!("Parent directory is full!");
    }

    // touch 命令
    pub fn touch(&mut self, path: &str) {
        // 1. 分离父路径和文件名
        let (parent_path, file_name) = match path.rsplit_once('/') {
            Some(("", name)) => ("/", name),
            Some((p, name)) => (p, name),
            None => {
                println!("touch: Path must start with /");
                return;
            }
        };

        // 2. 找到父目录 Inode
        let parent_ino = match self.resolve_path(parent_path) {
            Some(ino) => ino,
            None => {
                println!("touch: Parent directory not found");
                return;
            }
        };

        // 3. 分配一个新的 Inode
        let new_ino = self.alloc_inode().expect("No free Inodes");

        // 4. 初始化 Inode
        let new_inode = Inode {
            mode: InodeType::File,
            size: 0,
            blocks: [0; 12],
        };
        self.write_inode_to_disk(new_ino, &new_inode);

        // 5. 在父目录中增加条目
        self.add_dir_entry(parent_ino, new_ino, file_name);
        println!("File '{}' created.", file_name);
    }

    fn write_inode_to_disk(&mut self, ino: u32, inode: &Inode) {
        let mut table_buf = [0u8; BLOCK_SIZE];
        let block_idx = 3 + (ino * INODE_SIZE as u32) / BLOCK_SIZE as u32;
        let offset = (ino * INODE_SIZE as u32) % BLOCK_SIZE as u32;
        self.device.read_block(block_idx, &mut table_buf);
        table_buf[offset as usize..offset as usize + INODE_SIZE]
            .copy_from_slice(&inode.serialize());
        self.device.write_block(block_idx, &table_buf);
    }

    // write 命令
    pub fn write_file(&mut self, path: &str, content: &str) {
        // 1. 找到文件的 Inode
        let ino = match self.resolve_path(path) {
            Some(i) => i,
            None => {
                println!("write: File not found");
                return;
            }
        };

        let mut inode = self.read_inode(ino);
        if inode.mode != InodeType::File {
            println!("write: '{}' is not a file", path);
            return;
        }

        // 2. 分配数据块
        if inode.blocks[0] == 0 {
            let new_block = self.alloc_data_block().expect("No free Data Blocks");
            inode.blocks[0] = new_block;
        }

        // 3. 准备数据
        let bytes = content.as_bytes();
        let write_len = bytes.len().min(BLOCK_SIZE);
        let mut data_buf = [0u8; BLOCK_SIZE];
        data_buf[..write_len].copy_from_slice(&bytes[..write_len]);

        // 4. 写入数据块
        self.device.write_block(inode.blocks[0], &data_buf);

        // 5. 更新 Inode 大小并写回
        inode.size = write_len as u32;
        self.write_inode_to_disk(ino, &inode);

        println!("Wrote {} bytes to '{}'.", write_len, path);
    }

    // cat 命令
    pub fn cat(&mut self, path: &str) {
        let ino = match self.resolve_path(path) {
            Some(i) => i,
            None => {
                println!("cat: File not found");
                return;
            }
        };

        let inode = self.read_inode(ino);
        if inode.mode != InodeType::File {
            println!("cat: '{}' is not a file", path);
            return;
        }

        if inode.size == 0 {
            return;
        }

        // 读取第一个块
        let mut buf = [0u8; BLOCK_SIZE];
        self.device.read_block(inode.blocks[0], &mut buf);

        // 只打印 size 长度的内容
        let content = String::from_utf8_lossy(&buf[..inode.size as usize]);
        println!("{}", content);
    }

    fn set_bit(&mut self, bitmap_block_idx: u32, bit_idx: u32, val: bool) {
        let mut buf = [0u8; BLOCK_SIZE];
        self.device.read_block(bitmap_block_idx, &mut buf);
        let byte_pos = (bit_idx / 8) as usize;
        let bit_pos = (bit_idx % 8) as usize;
        if val {
            buf[byte_pos] |= 1 << bit_pos;
        } else {
            buf[byte_pos] &= !(1 << bit_pos);
        }
        self.device.write_block(bitmap_block_idx, &buf);
    }

    fn free_inode(&mut self, ino: u32) {
        self.set_bit(1, ino, false);
    }

    fn free_data_block(&mut self, block_idx: u32) {
        self.set_bit(2, block_idx, false);
    }

    fn remove_dir_entry(&mut self, parent_ino: u32, name: &str) {
        let mut parent_inode = self.read_inode(parent_ino);
        let mut buf = [0u8; BLOCK_SIZE];

        for i in 0..12 {
            let block_idx = parent_inode.blocks[i];
            if block_idx == 0 {
                continue;
            }
            self.device.read_block(block_idx, &mut buf);

            for j in 0..(BLOCK_SIZE / DIR_ENTRY_SIZE) {
                let start = j * DIR_ENTRY_SIZE;
                let entry = DirEntry::deserialize(&buf[start..start + DIR_ENTRY_SIZE]);
                if entry.inode_no != 0 && entry.name == name {
                    // 找到后抹除，写入一个全0的无效Entry
                    let empty_entry = [0u8; DIR_ENTRY_SIZE];
                    buf[start..start + DIR_ENTRY_SIZE].copy_from_slice(&empty_entry);
                    self.device.write_block(block_idx, &buf);
                    return;
                }
            }
        }
    }

    // rm 命令
    pub fn rm(&mut self, path: &str) {
        if path == "/" {
            println!("rm: Cannot remove root");
            return;
        }

        let ino = match self.resolve_path(path) {
            Some(i) => i,
            None => {
                println!("rm: File not found");
                return;
            }
        };

        let inode = self.read_inode(ino);

        // 简单起见，只支持了删空目录
        if inode.mode == InodeType::Directory {
            let mut count = 0;
            let mut buf = [0u8; BLOCK_SIZE];
            self.device.read_block(inode.blocks[0], &mut buf);
            for i in 0..(BLOCK_SIZE / DIR_ENTRY_SIZE) {
                let e = DirEntry::deserialize(&buf[i * DIR_ENTRY_SIZE..(i + 1) * DIR_ENTRY_SIZE]);
                if e.inode_no != 0 {
                    count += 1;
                }
            }
            if count > 2 {
                println!("rm: Directory not empty");
                return;
            }
        }

        // 1. 释放所有数据块
        for &b in inode.blocks.iter() {
            if b != 0 {
                self.free_data_block(b);
            }
        }

        // 2. 释放 Inode
        self.free_inode(ino);

        // 3. 删除父子关系
        let (parent_path, name) = path.rsplit_once('/').unwrap_or(("/", path));
        let parent_ino = self
            .resolve_path(if parent_path.is_empty() {
                "/"
            } else {
                parent_path
            })
            .unwrap();
        self.remove_dir_entry(parent_ino, name);

        println!("Removed '{}'", path);
    }

    // cp 命令
    pub fn cp(&mut self, src_path: &str, dst_path: &str) {
        let src_ino = match self.resolve_path(src_path) {
            Some(i) => i,
            None => {
                println!("cp: Source not found");
                return;
            }
        };
        let src_inode = self.read_inode(src_ino);
        if src_inode.mode != InodeType::File {
            println!("cp: Only file copying is supported");
            return;
        }

        // 其实就是 touch + write
        self.touch(dst_path);

        if src_inode.blocks[0] != 0 {
            let mut buf = [0u8; BLOCK_SIZE];
            self.device.read_block(src_inode.blocks[0], &mut buf);
            let content = String::from_utf8_lossy(&buf[..src_inode.size as usize]).to_string();
            self.write_file(dst_path, &content);
        }
    }

    // mv 命令
    pub fn mv(&mut self, src_path: &str, dst_path: &str) {
        let src_ino = match self.resolve_path(src_path) {
            Some(i) => i,
            None => {
                println!("mv: Source not found");
                return;
            }
        };

        let (dst_parent_path, dst_name) = dst_path.rsplit_once('/').unwrap_or(("/", dst_path));
        let dst_parent_ino = match self.resolve_path(if dst_parent_path.is_empty() {
            "/"
        } else {
            dst_parent_path
        }) {
            Some(i) => i,
            None => {
                println!("mv: Destination path invalid");
                return;
            }
        };

        // 其实也可以认为是 rm + write，但这样有不必要的开销，最好是直接更新相关索引而不动数据块
        self.add_dir_entry(dst_parent_ino, src_ino, dst_name);

        let (src_parent_path, src_name) = src_path.rsplit_once('/').unwrap_or(("/", src_path));
        let src_parent_ino = self
            .resolve_path(if src_parent_path.is_empty() {
                "/"
            } else {
                src_parent_path
            })
            .unwrap();
        self.remove_dir_entry(src_parent_ino, src_name);

        let mut inode = self.read_inode(src_ino);
        if inode.mode == InodeType::Directory {
            let mut buf = [0u8; BLOCK_SIZE];
            self.device.read_block(inode.blocks[0], &mut buf);
            let mut dotdot = DirEntry::deserialize(&buf[64..128]);
            dotdot.inode_no = dst_parent_ino;
            buf[64..128].copy_from_slice(&dotdot.serialize());
            self.device.write_block(inode.blocks[0], &buf);
        }

        println!("Moved '{}' to '{}'", src_path, dst_path);
    }
}
