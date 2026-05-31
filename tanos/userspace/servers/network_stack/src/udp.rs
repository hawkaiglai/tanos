//! UDP header parsing and writing

use crate::lib_extensions::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}

/// Thin wrapper around a protocol::Socket for UDP-specific operations.
pub type UdpSocket = crate::protocol::Socket;

impl UdpHeader {
    /// Parse a UDP header from raw bytes (minimum 8 bytes).
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(Error::ParseError);
        }

        Ok(Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            length: u16::from_be_bytes([data[4], data[5]]),
            checksum: u16::from_be_bytes([data[6], data[7]]),
        })
    }

    /// Write a UDP header into a buffer. Returns 8 (fixed header size).
    pub fn write(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 8 {
            return Err(Error::InvalidParameters);
        }

        buf[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        buf[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        buf[4..6].copy_from_slice(&self.length.to_be_bytes());
        buf[6..8].copy_from_slice(&self.checksum.to_be_bytes());

        Ok(8)
    }
}
