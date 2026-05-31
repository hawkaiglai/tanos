//! Filesystem abstractions for the VFS server

use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::lib_extensions::{Error, Result};
use crate::protocol::{FileStat, OpenFlags};

/// Types of VFS nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VNodeType {
    Regular,
    Directory,
    Symlink,
    Device,
}

/// Virtual node — represents a file or directory in the VFS
pub trait VNode: Send + Sync {
    /// Read data at the given offset
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize>;

    /// Write data at the given offset
    fn write(&self, offset: u64, buf: &[u8]) -> Result<usize>;

    /// Get file statistics
    fn stat(&self) -> Result<FileStat>;

    /// Get the node type
    fn node_type(&self) -> VNodeType;

    /// Clone this node as a trait object
    fn clone_vnode(&self) -> Arc<dyn VNode>;
}

/// Filesystem trait — implemented by each filesystem type
pub trait FileSystem: Send + Sync {
    /// Open a file by path
    fn open(&self, path: &str, flags: OpenFlags, mode: u32) -> Result<Arc<dyn VNode>>;

    /// Get file statistics by path
    fn stat(&self, path: &str) -> Result<FileStat>;

    /// Create a directory
    fn mkdir(&self, path: &str, mode: u32) -> Result<()>;

    /// List directory entries
    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>>;
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: [u8; 64],
    pub name_len: usize,
    pub node_type: VNodeType,
    pub inode: u64,
}

impl DirEntry {
    pub fn new(name: &str, node_type: VNodeType, inode: u64) -> Self {
        let mut buf = [0u8; 64];
        let len = name.len().min(63);
        buf[..len].copy_from_slice(&name.as_bytes()[..len]);
        Self {
            name: buf,
            name_len: len,
            node_type,
            inode,
        }
    }
}
