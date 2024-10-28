// Ignore unused functions while handshake isn't implemented
#![allow(dead_code)]

// Importações necessárias
use std::net::SocketAddr;

// use crate::config::BUFFER_SIZE;
use crate::flags::Flags;
use crate::header::Header;

#[derive(Clone)]
pub struct Packet {
    pub header: Header,
    pub data: Vec<u8>,
}

impl Packet {
    // Tamanho do buffer
    pub const BUFFER_SIZE: usize = 2<<9;
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr,
            origin: SocketAddr, seq_num: u32, is_last: bool,
            is_ack: bool, gossip: bool, is_syn: bool, is_fin: bool, data: Vec<u8>) -> Self {
        
        let flags = is_last & Flags::LST | is_ack & Flags::ACK | gossip & Flags::GSP | is_syn & Flags::SYN | is_fin & Flags::FIN;

        let mut header = Header::new(src_addr, dst_addr, origin, seq_num, flags, 0);
        let checksum = Self::checksum(&header, &data);
        header.checksum = checksum;
        Self { header, data }
    }

    pub fn get_ack(&self) -> Self {
        let ack_header = self.header.get_ack();
        Self {header: ack_header, data: Vec::new()}
    }

    pub fn get_syn(
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        seq_num: u32,
    ) -> Self {
        Self::new(
            src_addr,
            dst_addr,
            src_addr,
            seq_num,
            false,
            false,
            false,
            true,
            false,
            Vec::new(),
        ) 
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
        is_gossip: bool,
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
                is_gossip,
                false,
                false,
                chunk.to_vec(),
            )
        }).collect()
    }
}

impl std::fmt::Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Packet {}: {} -> {}, origin: {}", self.header.seq_num,
        self.header.src_addr.port(), self.header.dst_addr.port(), self.header.origin.port())
    }
}
