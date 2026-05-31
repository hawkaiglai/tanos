//! Network Stack Protocol Definitions

use kernel_types::ProcessId;
use alloc::vec::Vec;
use crate::lib_extensions::{Error, Result};

pub const SERVICE_NETWORK_STACK: u32 = 5;
pub const REGISTRY_SERVICE: u32 = 1;

#[repr(u32)]
pub enum NetworkOp {
    SocketCreate = 0x5000,
    SocketBind = 0x5001,
    SocketListen = 0x5002,
    SocketConnect = 0x5003,
    _SocketAccept = 0x5004,
    SocketSend = 0x5005,
    SocketRecv = 0x5006,
    SocketClose = 0x5007,
    InterfaceConfig = 0x5100,
    _RouteAdd = 0x5101,
    _RouteDelete = 0x5102,
    _GetStats = 0x5200,
}

impl NetworkOp {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x5000 => Some(Self::SocketCreate),
            0x5001 => Some(Self::SocketBind),
            0x5002 => Some(Self::SocketListen),
            0x5003 => Some(Self::SocketConnect),
            0x5005 => Some(Self::SocketSend),
            0x5006 => Some(Self::SocketRecv),
            0x5007 => Some(Self::SocketClose),
            0x5100 => Some(Self::InterfaceConfig),
            _ => None,
        }
    }
}

#[repr(u32)]
pub enum NetDriverOp {
    SendPacket = 0x6000,
    _ReceivePacket = 0x6001,
}

#[repr(u32)]
pub enum RegistryOp {
    Register = 0x100,
    _Lookup = 0x101,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SocketId(pub u32);

impl SocketId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream = 1,
    Datagram = 2,
    Raw = 3,
}

impl SocketType {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::Stream),
            2 => Some(Self::Datagram),
            3 => Some(Self::Raw),
            _ => None,
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Closed = 0,
    Listen = 1,
    SynSent = 2,
    SynReceived = 3,
    Established = 4,
    _FinWait1 = 5,
    _FinWait2 = 6,
    CloseWait = 7,
    _Closing = 8,
    _LastAck = 9,
    _TimeWait = 10,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SocketAddr {
    pub ip: IpAddress,
    pub port: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpAddress {
    pub octets: [u8; 4],
}

impl IpAddress {
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self { octets: [a, b, c, d] }
    }

    pub fn from_u32(addr: u32) -> Self {
        Self {
            octets: [
                (addr >> 24) as u8,
                (addr >> 16) as u8,
                (addr >> 8) as u8,
                addr as u8,
            ]
        }
    }
}

pub struct Socket {
    pub id: SocketId,
    pub socket_type: SocketType,
    pub _protocol: u32,
    pub state: SocketState,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub _owner: ProcessId,
    pub tcp_seq: u32,
    pub tcp_ack: u32,
    pub recv_buffer: Vec<u8>,
    pub _send_buffer: Vec<u8>,
}

impl Socket {
    pub fn new(id: SocketId, socket_type: SocketType, protocol: u32, owner: ProcessId) -> Self {
        Self {
            id,
            socket_type,
            _protocol: protocol,
            state: SocketState::Closed,
            local_addr: SocketAddr {
                ip: IpAddress::new(0, 0, 0, 0),
                port: 0,
            },
            remote_addr: SocketAddr {
                ip: IpAddress::new(0, 0, 0, 0),
                port: 0,
            },
            _owner: owner,
            tcp_seq: 1000,
            tcp_ack: 0,
            recv_buffer: Vec::new(),
            _send_buffer: Vec::new(),
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) -> Result<()> {
        if self.state != SocketState::Closed {
            return Err(Error::AlreadyBound);
        }
        self.local_addr = addr;
        Ok(())
    }

    pub fn listen(&mut self, _backlog: u32) -> Result<()> {
        if self.socket_type != SocketType::Stream {
            return Err(Error::InvalidOperation);
        }
        if self.local_addr.port == 0 {
            return Err(Error::NotBound);
        }
        self.state = SocketState::Listen;
        Ok(())
    }

    pub fn connect(&mut self, addr: SocketAddr) -> Result<()> {
        if self.state != SocketState::Closed {
            return Err(Error::AlreadyConnected);
        }
        self.remote_addr = addr;
        Ok(())
    }

    pub fn close(&mut self) {
        self.state = SocketState::Closed;
        self.recv_buffer.clear();
        self._send_buffer.clear();
    }
}
