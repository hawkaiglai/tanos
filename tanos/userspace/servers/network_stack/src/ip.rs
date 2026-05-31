//! IP header parsing and writing

pub use crate::protocol::IpAddress;
use crate::lib_extensions::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct IpHeader {
    pub version: u8,
    pub header_len: u8,
    pub type_of_service: u8,
    pub total_len: u16,
    pub identification: u16,
    pub flags: u8,
    pub fragment_offset: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src_addr: IpAddress,
    pub dst_addr: IpAddress,
}

impl IpHeader {
    /// Parse an IP header from raw bytes.
    /// Minimum: 20 bytes for a standard IPv4 header.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 20 {
            return Err(Error::ParseError);
        }

        let version = data[0] >> 4;
        let header_len = data[0] & 0x0F;
        let type_of_service = data[1];
        let total_len = u16::from_be_bytes([data[2], data[3]]);
        let identification = u16::from_be_bytes([data[4], data[5]]);
        let flags = data[6] >> 5;
        let fragment_offset = u16::from_be_bytes([data[6] & 0x1F, data[7]]);
        let ttl = data[8];
        let protocol = data[9];
        let checksum = u16::from_be_bytes([data[10], data[11]]);
        let src_addr = IpAddress::new(data[12], data[13], data[14], data[15]);
        let dst_addr = IpAddress::new(data[16], data[17], data[18], data[19]);

        Ok(Self {
            version,
            header_len,
            type_of_service,
            total_len,
            identification,
            flags,
            fragment_offset,
            ttl,
            protocol,
            checksum,
            src_addr,
            dst_addr,
        })
    }

    /// Write the IP header into a buffer. Returns 20 (fixed header size).
    pub fn write(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < 20 {
            return Err(Error::InvalidParameters);
        }

        buf[0] = (self.version << 4) | (self.header_len & 0x0F);
        buf[1] = self.type_of_service;
        buf[2..4].copy_from_slice(&self.total_len.to_be_bytes());
        buf[4..6].copy_from_slice(&self.identification.to_be_bytes());
        buf[6] = (self.flags << 5) | ((self.fragment_offset >> 8) as u8 & 0x1F);
        buf[7] = self.fragment_offset as u8;
        buf[8] = self.ttl;
        buf[9] = self.protocol;
        buf[10..12].copy_from_slice(&self.checksum.to_be_bytes());
        buf[12..16].copy_from_slice(&self.src_addr.octets);
        buf[16..20].copy_from_slice(&self.dst_addr.octets);

        Ok(20)
    }
}
