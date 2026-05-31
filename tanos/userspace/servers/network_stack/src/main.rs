#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate libmicro;

use alloc::{collections::BTreeMap, vec::Vec, sync::Arc};
use kernel_types::*;
use spin::Mutex;

mod lib_extensions;
mod protocol;
mod ethernet;
mod ip;
mod tcp;
mod udp;
mod socket;
mod packet_buffer;

use lib_extensions::{Error, Result, Message, MessageType, ipc, memory, syscall};
use protocol::*;
use ethernet::EthernetFrame;
use ip::IpHeader;
use tcp::{TcpSocket, TcpHeader};
use udp::{UdpSocket, UdpHeader};
use socket::{Socket, SocketType, SocketState};
use packet_buffer::PacketBuffer;

struct NetworkStack {
    endpoint: EndpointId,
    network_interface: Mutex<NetworkInterface>,
    sockets: Mutex<BTreeMap<SocketId, Arc<Mutex<Socket>>>>,
    routing_table: Mutex<Vec<RouteEntry>>,
    next_socket_id: Mutex<u32>,
    packet_pool: Mutex<Vec<PacketBuffer>>,
}

impl NetworkStack {
    fn new() -> Result<Self> {
        let endpoint = ipc::create_endpoint()?;

        // Register with system registry
        let registry = EndpointId::new_unchecked(REGISTRY_SERVICE);
        let mut msg = Message::new(MessageType::Call);
        msg.set_label(RegistryOp::Register as u32);
        msg.set_data(0, SERVICE_NETWORK_STACK as u64);
        msg.set_data(1, endpoint.as_u64());

        ipc::call(registry, &msg)?;

        // Initialize packet pool
        let mut packet_pool = Vec::new();
        for _ in 0..PACKET_POOL_SIZE {
            packet_pool.push(PacketBuffer::new());
        }

        Ok(Self {
            endpoint,
            network_interface: Mutex::new(NetworkInterface::new()),
            sockets: Mutex::new(BTreeMap::new()),
            routing_table: Mutex::new(Vec::new()),
            next_socket_id: Mutex::new(1),
            packet_pool: Mutex::new(packet_pool),
        })
    }

    fn allocate_socket_id(&self) -> SocketId {
        let mut next = self.next_socket_id.lock();
        let id = SocketId(*next);
        *next += 1;
        id
    }

    fn allocate_packet(&self) -> Option<PacketBuffer> {
        let mut pool = self.packet_pool.lock();
        pool.pop()
    }

    fn free_packet(&self, packet: PacketBuffer) {
        let mut pool = self.packet_pool.lock();
        if pool.len() < PACKET_POOL_SIZE {
            pool.push(packet);
        }
    }

    fn handle_socket_create(&self, msg: &Message) -> Message {
        let socket_type = SocketType::from_u32(msg.get_data(0) as u32);
        let protocol = msg.get_data(1) as u32;
        let process_id = ProcessId::new_const(msg.sender().as_u32() as u16);

        let socket_type = match socket_type {
            Some(t) => t,
            None => return Message::error(Error::InvalidParameters),
        };

        let socket_id = self.allocate_socket_id();
        let socket = Socket::new(socket_id, socket_type, protocol, process_id);

        {
            let mut sockets = self.sockets.lock();
            sockets.insert(socket_id, Arc::new(Mutex::new(socket)));
        }

        let mut reply = Message::new(MessageType::Reply);
        reply.set_data(0, socket_id.as_u64());
        reply
    }

    fn handle_socket_bind(&self, msg: &Message) -> Message {
        let socket_id = SocketId(msg.get_data(0) as u32);
        let addr_data = msg.get_data(1) as *const SocketAddr;

        let addr = unsafe { *addr_data };

        let sockets = self.sockets.lock();
        if let Some(socket_arc) = sockets.get(&socket_id) {
            let mut socket = socket_arc.lock();
            match socket.bind(addr) {
                Ok(()) => Message::success(),
                Err(e) => Message::error(e),
            }
        } else {
            Message::error(Error::SocketNotFound)
        }
    }

    fn handle_socket_listen(&self, msg: &Message) -> Message {
        let socket_id = SocketId(msg.get_data(0) as u32);
        let backlog = msg.get_data(1) as u32;

        let sockets = self.sockets.lock();
        if let Some(socket_arc) = sockets.get(&socket_id) {
            let mut socket = socket_arc.lock();
            match socket.listen(backlog) {
                Ok(()) => Message::success(),
                Err(e) => Message::error(e),
            }
        } else {
            Message::error(Error::SocketNotFound)
        }
    }

    fn handle_socket_connect(&self, msg: &Message) -> Message {
        let socket_id = SocketId(msg.get_data(0) as u32);
        let addr_data = msg.get_data(1) as *const SocketAddr;

        let addr = unsafe { *addr_data };

        let sockets = self.sockets.lock();
        if let Some(socket_arc) = sockets.get(&socket_id) {
            let mut socket = socket_arc.lock();
            match socket.connect(addr) {
                Ok(()) => {
                    // For TCP, initiate 3-way handshake
                    if socket.socket_type == SocketType::Stream {
                        if let Err(e) = self.tcp_connect(&mut socket, addr) {
                            return Message::error(e);
                        }
                    }
                    Message::success()
                }
                Err(e) => Message::error(e),
            }
        } else {
            Message::error(Error::SocketNotFound)
        }
    }

    fn handle_socket_send(&self, msg: &Message) -> Message {
        let socket_id = SocketId(msg.get_data(0) as u32);
        let buffer_shm = msg.get_data(1);
        let size = msg.get_data(2) as usize;
        let _flags = msg.get_data(3) as u32;

        // Map data buffer
        let buffer_ptr = match memory::map_shared_memory(buffer_shm) {
            Ok(ptr) => ptr,
            Err(e) => return Message::error(e),
        };

        let data = unsafe { core::slice::from_raw_parts(buffer_ptr as *const u8, size) };

        let sockets = self.sockets.lock();
        if let Some(socket_arc) = sockets.get(&socket_id) {
            let socket = socket_arc.lock();

            let bytes_sent = match socket.socket_type {
                SocketType::Stream => self.tcp_send(&socket, data),
                SocketType::Datagram => self.udp_send(&socket, data, socket.remote_addr),
                _ => return Message::error(Error::InvalidOperation),
            };

            match bytes_sent {
                Ok(n) => {
                    let mut reply = Message::new(MessageType::Reply);
                    reply.set_data(0, n as u64);
                    reply
                }
                Err(e) => Message::error(e),
            }
        } else {
            Message::error(Error::SocketNotFound)
        }
    }

    fn handle_socket_recv(&self, msg: &Message) -> Message {
        let socket_id = SocketId(msg.get_data(0) as u32);
        let buffer_shm = msg.get_data(1);
        let size = msg.get_data(2) as usize;
        let _flags = msg.get_data(3) as u32;

        // Map data buffer
        let buffer_ptr = match memory::map_shared_memory(buffer_shm) {
            Ok(ptr) => ptr,
            Err(e) => return Message::error(e),
        };

        let buffer = unsafe { core::slice::from_raw_parts_mut(buffer_ptr as *mut u8, size) };

        let sockets = self.sockets.lock();
        if let Some(socket_arc) = sockets.get(&socket_id) {
            let mut socket = socket_arc.lock();

            let bytes_received = match socket.socket_type {
                SocketType::Stream => self.tcp_recv(&mut socket, buffer),
                SocketType::Datagram => self.udp_recv(&mut socket, buffer),
                _ => return Message::error(Error::InvalidOperation),
            };

            match bytes_received {
                Ok(n) => {
                    let mut reply = Message::new(MessageType::Reply);
                    reply.set_data(0, n as u64);
                    reply
                }
                Err(e) => Message::error(e),
            }
        } else {
            Message::error(Error::SocketNotFound)
        }
    }

    fn handle_socket_close(&self, msg: &Message) -> Message {
        let socket_id = SocketId(msg.get_data(0) as u32);

        let mut sockets = self.sockets.lock();
        if let Some(socket_arc) = sockets.remove(&socket_id) {
            let mut socket = socket_arc.lock();
            socket.close();
            Message::success()
        } else {
            Message::error(Error::SocketNotFound)
        }
    }

    fn handle_interface_config(&self, msg: &Message) -> Message {
        let ip_addr = IpAddress::from_u32(msg.get_data(0) as u32);
        let netmask = IpAddress::from_u32(msg.get_data(1) as u32);
        let gateway = IpAddress::from_u32(msg.get_data(2) as u32);

        {
            let mut interface = self.network_interface.lock();
            interface.configure(ip_addr, netmask, gateway);
        }

        // Add default route
        {
            let mut routes = self.routing_table.lock();
            routes.push(RouteEntry {
                destination: IpAddress::new(0, 0, 0, 0),
                netmask: IpAddress::new(0, 0, 0, 0),
                gateway: Some(gateway),
                interface: 0,
                metric: 1,
            });
        }

        Message::success()
    }

    fn tcp_connect(&self, socket: &mut Socket, addr: SocketAddr) -> Result<()> {
        // Allocate packet for SYN
        let mut packet = self.allocate_packet().ok_or(Error::OutOfMemory)?;

        // Build TCP SYN packet
        let tcp_header = TcpHeader {
            src_port: socket.local_addr.port,
            dst_port: addr.port,
            seq_num: socket.tcp_seq,
            ack_num: 0,
            flags: TCP_SYN,
            window: TCP_DEFAULT_WINDOW,
            checksum: 0,
            urgent_ptr: 0,
        };

        // Build IP header
        let ip_header = IpHeader {
            version: 4,
            header_len: 5,
            type_of_service: 0,
            total_len: 40, // 20 IP + 20 TCP
            identification: 0,
            flags: 0,
            fragment_offset: 0,
            ttl: 64,
            protocol: IP_PROTOCOL_TCP,
            checksum: 0,
            src_addr: self.network_interface.lock().ip_addr,
            dst_addr: addr.ip,
        };

        packet.write_ip_header(&ip_header);
        packet.write_tcp_header(&tcp_header);

        // Send packet
        self.send_packet(packet)?;

        // Update socket state
        socket.state = SocketState::SynSent;
        socket.tcp_seq += 1;

        Ok(())
    }

    fn tcp_send(&self, socket: &Socket, data: &[u8]) -> Result<usize> {
        if socket.state != SocketState::Established {
            return Err(Error::NotConnected);
        }

        let mut bytes_sent = 0;
        let mut remaining = data;

        while !remaining.is_empty() {
            let chunk_size = remaining.len().min(TCP_MAX_SEGMENT_SIZE);
            let chunk = &remaining[..chunk_size];

            // Allocate packet
            let mut packet = self.allocate_packet().ok_or(Error::OutOfMemory)?;

            // Build TCP header
            let tcp_header = TcpHeader {
                src_port: socket.local_addr.port,
                dst_port: socket.remote_addr.port,
                seq_num: socket.tcp_seq + bytes_sent as u32,
                ack_num: socket.tcp_ack,
                flags: TCP_PSH | TCP_ACK,
                window: TCP_DEFAULT_WINDOW,
                checksum: 0,
                urgent_ptr: 0,
            };

            // Build IP header
            let ip_header = IpHeader {
                version: 4,
                header_len: 5,
                type_of_service: 0,
                total_len: (20 + 20 + chunk_size) as u16,
                identification: 0,
                flags: 0,
                fragment_offset: 0,
                ttl: 64,
                protocol: IP_PROTOCOL_TCP,
                checksum: 0,
                src_addr: self.network_interface.lock().ip_addr,
                dst_addr: socket.remote_addr.ip,
            };

            packet.write_ip_header(&ip_header);
            packet.write_tcp_header(&tcp_header);
            packet.write_data(chunk);

            // Send packet
            self.send_packet(packet)?;

            bytes_sent += chunk_size;
            remaining = &remaining[chunk_size..];
        }

        Ok(bytes_sent)
    }

    fn tcp_recv(&self, socket: &mut Socket, buffer: &mut [u8]) -> Result<usize> {
        if socket.state != SocketState::Established {
            return Err(Error::NotConnected);
        }

        let bytes_to_copy = socket.recv_buffer.len().min(buffer.len());
        buffer[..bytes_to_copy].copy_from_slice(&socket.recv_buffer[..bytes_to_copy]);
        socket.recv_buffer.drain(..bytes_to_copy);

        Ok(bytes_to_copy)
    }

    fn udp_send(&self, socket: &Socket, data: &[u8], dest: SocketAddr) -> Result<usize> {
        // Allocate packet
        let mut packet = self.allocate_packet().ok_or(Error::OutOfMemory)?;

        // Build UDP header
        let udp_header = UdpHeader {
            src_port: socket.local_addr.port,
            dst_port: dest.port,
            length: (8 + data.len()) as u16,
            checksum: 0,
        };

        // Build IP header
        let ip_header = IpHeader {
            version: 4,
            header_len: 5,
            type_of_service: 0,
            total_len: (20 + 8 + data.len()) as u16,
            identification: 0,
            flags: 0,
            fragment_offset: 0,
            ttl: 64,
            protocol: IP_PROTOCOL_UDP,
            checksum: 0,
            src_addr: self.network_interface.lock().ip_addr,
            dst_addr: dest.ip,
        };

        packet.write_ip_header(&ip_header);
        packet.write_udp_header(&udp_header);
        packet.write_data(data);

        // Send packet
        self.send_packet(packet)?;

        Ok(data.len())
    }

    fn udp_recv(&self, socket: &mut Socket, buffer: &mut [u8]) -> Result<usize> {
        let bytes_to_copy = socket.recv_buffer.len().min(buffer.len());
        buffer[..bytes_to_copy].copy_from_slice(&socket.recv_buffer[..bytes_to_copy]);
        socket.recv_buffer.drain(..bytes_to_copy);

        Ok(bytes_to_copy)
    }

    fn send_packet(&self, packet: PacketBuffer) -> Result<()> {
        // Get network driver endpoint
        let net_driver = EndpointId::new_unchecked(NETWORK_DRIVER_SERVICE);

        // Send packet to network driver
        let mut msg = Message::new(MessageType::Call);
        msg.set_label(NetDriverOp::SendPacket as u32);
        msg.set_data(0, packet.as_ptr() as u64);
        msg.set_data(1, packet.len() as u64);

        ipc::call(net_driver, &msg)?;

        // Return packet to pool
        self.free_packet(packet);

        Ok(())
    }

    fn process_incoming_packet(&self, packet_data: &[u8]) -> Result<()> {
        // Parse Ethernet frame
        let eth_frame = EthernetFrame::parse(packet_data)?;

        match eth_frame.ethertype {
            ETHERTYPE_IP => {
                self.process_ip_packet(eth_frame.payload)?;
            }
            ETHERTYPE_ARP => {
                self.process_arp_packet(eth_frame.payload)?;
            }
            _ => {
                // Unknown protocol, drop packet
            }
        }

        Ok(())
    }

    fn process_ip_packet(&self, ip_data: &[u8]) -> Result<()> {
        let ip_header = IpHeader::parse(ip_data)?;

        // Check if packet is for us
        let our_ip = self.network_interface.lock().ip_addr;
        if ip_header.dst_addr != our_ip {
            return Ok(()); // Not for us
        }

        let payload = &ip_data[ip_header.header_len as usize * 4..];

        match ip_header.protocol {
            IP_PROTOCOL_TCP => {
                self.process_tcp_packet(&ip_header, payload)?;
            }
            IP_PROTOCOL_UDP => {
                self.process_udp_packet(&ip_header, payload)?;
            }
            IP_PROTOCOL_ICMP => {
                self.process_icmp_packet(&ip_header, payload)?;
            }
            _ => {
                // Unknown protocol
            }
        }

        Ok(())
    }

    fn process_tcp_packet(&self, ip_header: &IpHeader, tcp_data: &[u8]) -> Result<()> {
        let tcp_header = TcpHeader::parse(tcp_data)?;

        let sockets = self.sockets.lock();
        for socket_arc in sockets.values() {
            let mut socket = socket_arc.lock();

            if socket.local_addr.port == tcp_header.dst_port {
                self.handle_tcp_state_machine(&mut socket, &tcp_header, &tcp_data[20..])?;
                break;
            }
        }

        Ok(())
    }

    fn handle_tcp_state_machine(&self, socket: &mut Socket, tcp_header: &TcpHeader, data: &[u8]) -> Result<()> {
        match socket.state {
            SocketState::Listen => {
                if tcp_header.flags & TCP_SYN != 0 {
                    socket.tcp_ack = tcp_header.seq_num + 1;
                    socket.state = SocketState::SynReceived;
                }
            }
            SocketState::SynSent => {
                if tcp_header.flags & (TCP_SYN | TCP_ACK) == (TCP_SYN | TCP_ACK) {
                    socket.tcp_ack = tcp_header.seq_num + 1;
                    socket.state = SocketState::Established;
                }
            }
            SocketState::Established => {
                if !data.is_empty() {
                    socket.recv_buffer.extend_from_slice(data);
                    socket.tcp_ack += data.len() as u32;
                }

                if tcp_header.flags & TCP_FIN != 0 {
                    socket.state = SocketState::CloseWait;
                    socket.tcp_ack += 1;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn process_udp_packet(&self, _ip_header: &IpHeader, udp_data: &[u8]) -> Result<()> {
        let udp_header = UdpHeader::parse(udp_data)?;
        let payload = &udp_data[8..];

        let sockets = self.sockets.lock();
        for socket_arc in sockets.values() {
            let mut socket = socket_arc.lock();

            if socket.socket_type == SocketType::Datagram &&
               socket.local_addr.port == udp_header.dst_port {
                socket.recv_buffer.extend_from_slice(payload);
                break;
            }
        }

        Ok(())
    }

    fn process_icmp_packet(&self, ip_header: &IpHeader, icmp_data: &[u8]) -> Result<()> {
        if icmp_data.len() >= 8 && icmp_data[0] == 8 { // Echo request
            let mut packet = self.allocate_packet().ok_or(Error::OutOfMemory)?;

            // Build ICMP echo reply
            let mut icmp_reply = Vec::new();
            icmp_reply.push(0); // Echo reply type
            icmp_reply.push(0); // Code
            icmp_reply.extend_from_slice(&[0, 0]); // Checksum placeholder
            icmp_reply.extend_from_slice(&icmp_data[4..]); // Rest of packet

            // Calculate checksum
            let checksum = self.calculate_checksum(&icmp_reply);
            icmp_reply[2] = (checksum >> 8) as u8;
            icmp_reply[3] = checksum as u8;

            // Build IP reply
            let ip_reply = IpHeader {
                version: 4,
                header_len: 5,
                type_of_service: 0,
                total_len: (20 + icmp_reply.len()) as u16,
                identification: 0,
                flags: 0,
                fragment_offset: 0,
                ttl: 64,
                protocol: IP_PROTOCOL_ICMP,
                checksum: 0,
                src_addr: self.network_interface.lock().ip_addr,
                dst_addr: ip_header.src_addr,
            };

            packet.write_ip_header(&ip_reply);
            packet.write_data(&icmp_reply);

            self.send_packet(packet)?;
        }

        Ok(())
    }

    fn process_arp_packet(&self, _arp_data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn calculate_checksum(&self, data: &[u8]) -> u16 {
        let mut sum = 0u32;
        let mut i = 0;

        while i + 1 < data.len() {
            sum += ((data[i] as u32) << 8) + (data[i + 1] as u32);
            i += 2;
        }

        if i < data.len() {
            sum += (data[i] as u32) << 8;
        }

        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    fn run(&self) -> ! {
        debug_println!("Network stack started");

        loop {
            let mut msg = Message::new(MessageType::Receive);

            match ipc::receive(self.endpoint, &mut msg) {
                Ok(()) => {
                    let reply = match NetworkOp::from_u32(msg.label()) {
                        Some(NetworkOp::SocketCreate) => self.handle_socket_create(&msg),
                        Some(NetworkOp::SocketBind) => self.handle_socket_bind(&msg),
                        Some(NetworkOp::SocketListen) => self.handle_socket_listen(&msg),
                        Some(NetworkOp::SocketConnect) => self.handle_socket_connect(&msg),
                        Some(NetworkOp::SocketSend) => self.handle_socket_send(&msg),
                        Some(NetworkOp::SocketRecv) => self.handle_socket_recv(&msg),
                        Some(NetworkOp::SocketClose) => self.handle_socket_close(&msg),
                        Some(NetworkOp::InterfaceConfig) => self.handle_interface_config(&msg),
                        _ => Message::error(Error::InvalidOperation),
                    };

                    let _ = ipc::reply(msg.sender(), &reply);
                }
                Err(e) => {
                    debug_println!("Network stack receive error: {:?}", e);
                }
            }
        }
    }
}

#[derive(Clone)]
struct NetworkInterface {
    ip_addr: IpAddress,
    netmask: IpAddress,
    gateway: IpAddress,
    _mac_addr: [u8; 6],
}

impl NetworkInterface {
    fn new() -> Self {
        Self {
            ip_addr: IpAddress::new(0, 0, 0, 0),
            netmask: IpAddress::new(0, 0, 0, 0),
            gateway: IpAddress::new(0, 0, 0, 0),
            _mac_addr: [0; 6],
        }
    }

    fn configure(&mut self, ip: IpAddress, netmask: IpAddress, gateway: IpAddress) {
        self.ip_addr = ip;
        self.netmask = netmask;
        self.gateway = gateway;
    }
}

struct RouteEntry {
    destination: IpAddress,
    netmask: IpAddress,
    gateway: Option<IpAddress>,
    interface: u32,
    metric: u32,
}

// Constants
const PACKET_POOL_SIZE: usize = 1024;
const TCP_MAX_SEGMENT_SIZE: usize = 1460;
const TCP_DEFAULT_WINDOW: u16 = 8192;
const TCP_SYN: u8 = 0x02;
const TCP_ACK: u8 = 0x10;
const TCP_PSH: u8 = 0x08;
const TCP_FIN: u8 = 0x01;
const ETHERTYPE_IP: u16 = 0x0800;
const ETHERTYPE_ARP: u16 = 0x0806;
const IP_PROTOCOL_TCP: u8 = 6;
const IP_PROTOCOL_UDP: u8 = 17;
const IP_PROTOCOL_ICMP: u8 = 1;
const NETWORK_DRIVER_SERVICE: u32 = 10;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    libmicro::init();

    let stack = NetworkStack::new().expect("Failed to create network stack");
    stack.run()
}
