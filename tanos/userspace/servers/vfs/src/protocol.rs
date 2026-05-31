//! VFS Server Protocol Definitions

use bitflags::bitflags;

pub const SERVICE_VFS: u32 = 4;
pub const REGISTRY_SERVICE: u32 = 1;

#[repr(u32)]
pub enum VfsOp {
    Open = 0x4000,
    Close = 0x4001,
    Read = 0x4002,
    Write = 0x4003,
    Seek = 0x4004,
    Stat = 0x4005,
    Mkdir = 0x4100,
    _Rmdir = 0x4101,
    _Readdir = 0x4102,
    Mount = 0x4200,
    _Unmount = 0x4201,
}

impl VfsOp {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x4000 => Some(Self::Open),
            0x4001 => Some(Self::Close),
            0x4002 => Some(Self::Read),
            0x4003 => Some(Self::Write),
            0x4004 => Some(Self::Seek),
            0x4005 => Some(Self::Stat),
            0x4100 => Some(Self::Mkdir),
            0x4200 => Some(Self::Mount),
            _ => None,
        }
    }
}

#[repr(u32)]
pub enum RegistryOp {
    Register = 0x100,
    _Lookup = 0x101,
    _Unregister = 0x102,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u32 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const APPEND = 0b00000100;
        const CREATE = 0b00001000;
        const TRUNCATE = 0b00010000;
        const EXCLUSIVE = 0b00100000;
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum SeekWhence {
    Set = 0,
    Current = 1,
    End = 2,
}

impl SeekWhence {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Set),
            1 => Some(Self::Current),
            2 => Some(Self::End),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileStat {
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub inode: u64,
    pub device: u64,
    pub links: u32,
    pub block_size: u32,
    pub blocks: u64,
}

#[repr(u32)]
pub enum FileType {
    Regular = 0,
    Directory = 1,
    Symlink = 2,
    Device = 3,
    _Pipe = 4,
    _Socket = 5,
}
