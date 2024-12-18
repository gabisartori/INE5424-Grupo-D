// Importações necessárias
use std::net::SocketAddr;
use std::fmt::{self, Debug, Display, Formatter};

// use crate::config::BUFFER_SIZE;
use crate::flags::Flags;
use crate::header::Header;
use crate::node::Node;

#[derive(Clone)]
pub struct Packet {
    pub header: Header,
    pub data: Vec<u8>,
}

impl Packet {
    // Tamanho do buffer
    pub const BUFFER_SIZE: usize = 2<<9;
    pub fn new(
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        origin: SocketAddr,
        seq_num: u32,
        is_last: bool,
        is_ack: bool,
        is_brd: bool,
        data: Vec<u8>
    ) -> Self {
        let mut flags:Flags = {
            Flags::EMP
        };
        if is_last {
            flags = flags | Flags::LST;
        }
        if is_ack {
            flags = flags | Flags::ACK
        }
        if is_brd  {
            flags = flags | Flags::BRD
        };

        let mut header = Header::new(src_addr, dst_addr, origin, seq_num, flags, 0);
        let checksum = Self::checksum(&header, &data);
        header.checksum = checksum;
        Self { header, data }
    }

    pub fn heart_beat(host: &Node, dst_addr: SocketAddr) -> Self {
        let mut header = Header::new(host.addr, dst_addr, host.addr, host.agent_number as u32, Flags::HB, 0);
        header.checksum = Header::checksum(&header);
        Self { header, data: Vec::new() }
    }

    pub fn get_ack(&self) -> Self {
        let ack_header = self.header.get_ack();
        Self {header: ack_header, data: Vec::new()}
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.data);
        bytes
    }

    pub fn from_bytes(bytes: [u8; Packet::BUFFER_SIZE], data_size: usize) -> Result<Self, std::array::TryFromSliceError> {
        let header = Header::from_bytes(bytes[..Header::HEADER_SIZE].try_into()?);
        let data = bytes[Header::HEADER_SIZE..data_size].to_vec();
        Ok(Self { header, data })
    }

    pub fn checksum(header: &Header, data: &Vec<u8>) -> u32 {
        let mut sum = Header::checksum(header);
        for byte in data {
            sum = sum.wrapping_add(*byte as u32);
        }
        sum
    }

    pub fn packets_from_message(
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        origin: SocketAddr,
        data: Vec<u8>,
        seq_num: u32,
        is_brd: bool,
    ) -> Vec<Self> {
        let chunks: Vec<&[u8]> = data.chunks(Packet::BUFFER_SIZE - Header::HEADER_SIZE).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            Packet::new(
                src_addr,
                dst_addr,
                origin,
                seq_num + i as u32,
                i == (chunks.len() - 1),
                false,
                is_brd,
                chunk.to_vec(),
            )
        }).collect()
    }
}

impl Debug for Packet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Packet {}: {} -> {}, origin: {}", self.header.seq_num,
        self.header.src_addr.port() % 100, self.header.dst_addr.port() % 100, self.header.origin.port() % 100)
    }
}

impl Display for Packet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let pkt = if self.header.is_ack() { "ACK" } else if self.header.is_heartbeat() {
            "Heartbeat"} else if self.header.is_brd() {
            "Broadcast" } else { "Packet" };
        write!(f, "{pkt} num {}: Agent {} -> Agent {}, origin: {}", self.header.seq_num,
        self.header.src_addr.port() % 100, self.header.dst_addr.port() % 100, self.header.origin.port() % 100)
    }
}
