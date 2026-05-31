//! File table management for the VFS server

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use kernel_types::ProcessId;
use crate::filesystem::VNode;
use crate::protocol::OpenFlags;

/// A handle to an open file
pub struct FileHandle {
    pub fd: u32,
    pub flags: OpenFlags,
    pub position: u64,
    pub vnode: Arc<dyn VNode>,
    pub process_id: ProcessId,
}

/// File table tracking all open file descriptors per process
pub struct FileTable {
    entries: BTreeMap<(u16, u32), FileHandle>,
}

impl FileTable {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, process_id: ProcessId, fd: u32, handle: FileHandle) {
        self.entries.insert((process_id.as_u16(), fd), handle);
    }

    pub fn get_mut(&mut self, process_id: ProcessId, fd: u32) -> Option<&mut FileHandle> {
        self.entries.get_mut(&(process_id.as_u16(), fd))
    }

    pub fn remove(&mut self, process_id: ProcessId, fd: u32) -> Option<FileHandle> {
        self.entries.remove(&(process_id.as_u16(), fd))
    }

    pub fn _len(&self) -> usize {
        self.entries.len()
    }
}
