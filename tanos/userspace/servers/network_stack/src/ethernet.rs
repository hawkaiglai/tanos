//! Ethernet frame parsing

use crate::lib_extensions::{Error, Result};

#[derive(Debug, Clone)]
pub struct EthernetFrame<'a> {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ethertype: u16,
    pub payload: &'a [u8],
}

impl<'a> EthernetFrame<'a> {
    /// Parse an Ethernet frame from raw bytes.
    /// Minimum frame: 14-byte header (6 dst + 6 src + 2 ethertype).
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < 14 {
            return Err(Error::ParseError);
        }

        let mut dst_mac = [0u8; 6];
        let mut src_mac = [0u8; 6];
        dst_mac.copy_from_slice(&data[0..6]);
        src_mac.copy_from_slice(&data[6..12]);
        let ethertype = u16::from_be_bytes([data[12], data[13]]);

        Ok(Self {
            dst_mac,
            src_mac,
            ethertype,
            payload: &data[14..],
        })
    }

    /// Write an Ethernet frame into a buffer. Returns bytes written.
    pub fn write(&self, buf: &mut [u8]) -> Result<usize> {
        let total = 14 + self.payload.len();
        if buf.len() < total {
            return Err(Error::InvalidParameters);
        }

        buf[0..6].copy_from_slice(&self.dst_mac);
        buf[6..12].copy_from_slice(&self.src_mac);
        buf[12..14].copy_from_slice(&self.ethertype.to_be_bytes());
        buf[14..total].copy_from_slice(self.payload);

        Ok(total)
    }
}
