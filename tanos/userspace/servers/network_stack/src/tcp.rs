//! TCP header parsing and writing

use crate::lib_extensions::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub flags: u8,
    pub window: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
}

/// Thin wrapper around a protocol::Socket for TCP-specific operations.
pub type TcpSocket = crate::protocol::Socket;

impl TcpHeader {
    /// Parse a TCP header from raw bytes (minimum 20 bytes).
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 20 {
            return Err(Error::ParseError);
        }

        Ok(Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            seq_num: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            ack_num: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            flags: data[13],
            window: u16::from_be_bytes([data[14], data[15]]),
            checksum: u16::from_be_bytes([data[16], data[17]]),
            urgent_ptr: u16::from_be_bytes([data[18], data[19]]),
        })
    }

    /// Write a TCP header into a buffer. Returns 20 (fixed header size).
    pub fn write(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 20 {
            return Err(Error::InvalidParameters);
        }

        buf[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        buf[4..8].copy_from_slice(&self.seq_num.to_be_bytes());
        buf[8..12].copy_from_slice(&self.ack_num.to_be_bytes());
        buf[12] = 5 << 4; // data offset = 5 (20 bytes), no options
        buf[13] = self.flags;
        buf[14..16].copy_from_slice(&self.window.to_be_bytes());
        buf[16..18].copy_from_slice(&self.checksum.to_be_bytes());
        buf[18..20].copy_from_slice(&self.urgent_ptr.to_be_bytes());

        Ok(20)
    }
}
