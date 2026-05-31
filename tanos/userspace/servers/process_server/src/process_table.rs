use alloc::collections::BTreeMap;
use kernel_types::ProcessId;
use crate::protocol::ProcessInfo;

pub struct ProcessTable {
    processes: BTreeMap<ProcessId, ProcessInfo>,
}

impl ProcessTable {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, pid: ProcessId, info: ProcessInfo) {
        self.processes.insert(pid, info);
    }

    pub fn remove(&mut self, pid: ProcessId) -> Option<ProcessInfo> {
        self.processes.remove(&pid)
    }

    pub fn get(&self, pid: ProcessId) -> Option<&ProcessInfo> {
        self.processes.get(&pid)
    }

    pub fn get_mut(&mut self, pid: ProcessId) -> Option<&mut ProcessInfo> {
        self.processes.get_mut(&pid)
    }

    pub fn len(&self) -> usize {
        self.processes.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ProcessId, &ProcessInfo)> {
        self.processes.iter()
    }
}
