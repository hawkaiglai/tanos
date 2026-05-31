//! Packet buffer for network I/O

use crate::ip::IpHeader;
use crate::tcp::TcpHeader;
use crate::udp::UdpHeader;

/// Maximum Ethernet frame size (MTU 1500 + 14 header + safety margin).
const MAX_PACKET_SIZE: usize = 1536;

/// Fixed-size buffer for assembling and parsing network packets.
pub struct PacketBuffer {
    data: [u8; MAX_PACKET_SIZE],
    len: usize,
}

impl PacketBuffer {
    /// Create an empty packet buffer.
    pub fn new() -> Self {
        Self {
            data: [0u8; MAX_PACKET_SIZE],
            len: 0,
        }
    }

    /// Write an IP header at the current position.
    pub fn write_ip_header(&mut self, header: &IpHeader) {
        if let Ok(n) = header.write(&mut self.data[self.len..]) {
            self.len += n;
        }
    }

    /// Write a TCP header at the current position.
    pub fn write_tcp_header(&mut self, header: &TcpHeader) {
        if let Ok(n) = header.write(&mut self.data[self.len..]) {
            self.len += n;
        }
    }

    /// Write a UDP header at the current position.
    pub fn write_udp_header(&mut self, header: &UdpHeader) {
        if let Ok(n) = header.write(&mut self.data[self.len..]) {
            self.len += n;
        }
    }

    /// Write raw payload data at the current position.
    pub fn write_data(&mut self, data: &[u8]) {
        let available = MAX_PACKET_SIZE - self.len;
        let copy_len = data.len().min(available);
        self.data[self.len..self.len + copy_len].copy_from_slice(&data[..copy_len]);
        self.len += copy_len;
    }

    /// Return a pointer to the underlying buffer.
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Return the number of bytes written so far.
    pub fn len(&self) -> usize {
        self.len
    }
}
