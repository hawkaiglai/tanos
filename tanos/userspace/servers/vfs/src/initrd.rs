//! Initial RAM Disk filesystem implementation

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::RwLock;

use crate::filesystem::{FileSystem, VNode, VNodeType, DirEntry};
use crate::protocol::{FileStat, OpenFlags};
use crate::lib_extensions::{Error, Result};

/// In-memory filesystem backed by initial RAM disk data
pub struct InitrdFS {
    files: RwLock<BTreeMap<String, InitrdFile>>,
}

struct InitrdFile {
    data: Vec<u8>,
    node_type: VNodeType,
    inode: u64,
}

/// VNode implementation for initrd files
struct InitrdVNode {
    data: Vec<u8>,
    node_type: VNodeType,
    inode: u64,
}

impl InitrdFS {
    pub fn new() -> Result<Self> {
        let mut files = BTreeMap::new();

        // Create root directory
        files.insert(String::from("/"), InitrdFile {
            data: Vec::new(),
            node_type: VNodeType::Directory,
            inode: 1,
        });

        // Create some default files for the demo
        files.insert(String::from("/readme.txt"), InitrdFile {
            data: Vec::from(b"Welcome to TanOS!\nThis is the initial RAM disk.\n" as &[u8]),
            node_type: VNodeType::Regular,
            inode: 2,
        });

        files.insert(String::from("/version"), InitrdFile {
            data: Vec::from(b"TanOS v3.0.0\n" as &[u8]),
            node_type: VNodeType::Regular,
            inode: 3,
        });

        Ok(Self {
            files: RwLock::new(files),
        })
    }
}

impl FileSystem for InitrdFS {
    fn open(&self, path: &str, _flags: OpenFlags, _mode: u32) -> Result<Arc<dyn VNode>> {
        let files = self.files.read();
        let file = files.get(path).ok_or(Error::FileNotFound)?;

        Ok(Arc::new(InitrdVNode {
            data: file.data.clone(),
            node_type: file.node_type,
            inode: file.inode,
        }))
    }

    fn stat(&self, path: &str) -> Result<FileStat> {
        let files = self.files.read();
        let file = files.get(path).ok_or(Error::FileNotFound)?;

        Ok(FileStat {
            size: file.data.len() as u64,
            mode: if file.node_type == VNodeType::Directory { 0o755 } else { 0o644 },
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            inode: file.inode,
            device: 0,
            links: 1,
            block_size: 512,
            blocks: ((file.data.len() + 511) / 512) as u64,
        })
    }

    fn mkdir(&self, path: &str, _mode: u32) -> Result<()> {
        let mut files = self.files.write();

        if files.contains_key(path) {
            return Err(Error::AlreadyExists);
        }

        let inode = files.len() as u64 + 1;
        files.insert(String::from(path), InitrdFile {
            data: Vec::new(),
            node_type: VNodeType::Directory,
            inode,
        });

        Ok(())
    }

    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let files = self.files.read();

        // Verify path is a directory
        if let Some(dir) = files.get(path) {
            if dir.node_type != VNodeType::Directory {
                return Err(Error::NotADirectory);
            }
        } else {
            return Err(Error::FileNotFound);
        }

        let prefix = if path == "/" { String::from("/") } else { format!("{}/", path) };
        let mut entries = Vec::new();

        for (file_path, file) in files.iter() {
            if file_path == path {
                continue;
            }
            // Check if file is a direct child of this directory
            if file_path.starts_with(prefix.as_str()) {
                let relative = &file_path[prefix.len()..];
                if !relative.contains('/') {
                    entries.push(DirEntry::new(relative, file.node_type, file.inode));
                }
            }
        }

        Ok(entries)
    }
}

impl VNode for InitrdVNode {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize> {
        if self.node_type == VNodeType::Directory {
            return Err(Error::IsADirectory);
        }

        let offset = offset as usize;
        if offset >= self.data.len() {
            return Ok(0);
        }

        let available = self.data.len() - offset;
        let to_copy = buf.len().min(available);
        buf[..to_copy].copy_from_slice(&self.data[offset..offset + to_copy]);
        Ok(to_copy)
    }

    fn write(&self, _offset: u64, _buf: &[u8]) -> Result<usize> {
        // InitrdFS is read-only
        Err(Error::PermissionDenied)
    }

    fn stat(&self) -> Result<FileStat> {
        Ok(FileStat {
            size: self.data.len() as u64,
            mode: if self.node_type == VNodeType::Directory { 0o755 } else { 0o644 },
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            inode: self.inode,
            device: 0,
            links: 1,
            block_size: 512,
            blocks: ((self.data.len() + 511) / 512) as u64,
        })
    }

    fn node_type(&self) -> VNodeType {
        self.node_type
    }

    fn clone_vnode(&self) -> Arc<dyn VNode> {
        Arc::new(InitrdVNode {
            data: self.data.clone(),
            node_type: self.node_type,
            inode: self.inode,
        })
    }
}

use alloc::format;
