// Importações necessárias
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// Tamanho do buffer
use crate::config::{CORRUPTION_RATE, BUFFER_SIZE};
use super::flags::Flags;

#[derive(Clone)]
pub struct Packet {
    pub header: Header,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr, origin: SocketAddr,
            seq_num: u32, is_last: bool,
            is_ack: bool, gossip: bool, data: Vec<u8>) -> Self {
        let mut flags = Flags::EMP;
        
        if is_last {
            flags = flags | Flags::LST;
        }
        if is_ack {
            flags = flags | Flags::ACK;
        }
        if gossip {
            flags = flags | Flags::GSP;
        }
        let checksum = Self::checksum(&Header::new(src_addr, dst_addr, origin, seq_num, flags, 0), &data);
        let header = Header::new(src_addr, dst_addr, origin, seq_num, flags, checksum);
        Self { header, data }
    }

    pub fn get_ack(&self) -> Self {
        let ack_header = self.header.get_ack();
        Self {header: ack_header, data: Vec::new()}  
    }

    // pub fn get_resend(&self, new_dst: SocketAddr) -> Self {
    //     let mut new_header = Header{
    //         src_addr: self.header.dst_addr,
    //         dst_addr: new_dst,
    //         origin: self.header.origin,
    //         seq_num: self.header.seq_num,
    //         flags: self.header.flags,
    //         checksum: 0,
    //     };
    //     let checksum = Self::checksum(&new_header, &self.data);
    //     new_header.checksum = checksum;
    //     Self {header: new_header, data: self.data.clone()}
    // }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.data);
        bytes
    }

    pub fn from_bytes(bytes: [u8; BUFFER_SIZE], data_size: usize) -> Self {
        let header = Header::from_bytes(bytes[..Header::HEADER_SIZE].try_into().unwrap());
        let data = bytes[Header::HEADER_SIZE..data_size].to_vec();
        Self { header, data }
    }

    fn sum_addr(addr: SocketAddr) -> u32 {
        let value = match addr.ip() {
            IpAddr::V4(ipv4) => u32::from_be_bytes(ipv4.octets()),
            IpAddr::V6(ipv6) => u128::from_be_bytes(ipv6.octets()) as u32,
        };
        value.wrapping_add(addr.port() as u32)
    }

    pub fn checksum(header: &Header, data: &Vec<u8>) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Self::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.origin));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum = sum.wrapping_add(header.flags.value as u32);
        for byte in data {
            sum = sum.wrapping_add(*byte as u32);
        }
        if rand::random::<f32>() < CORRUPTION_RATE {
            sum = sum.wrapping_add(1);
        }
        sum
    }

    pub fn packets_from_message(
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        origin: SocketAddr,
        data: Vec<u8>,
        start_sequence_number: u32,
        is_gossip: bool,
    ) -> Vec<Self> {
        let chunks: Vec<&[u8]> = data.chunks(BUFFER_SIZE - Header::HEADER_SIZE).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            Packet::new(
                src_addr,
                dst_addr,
                origin,
                start_sequence_number + i as u32,
                i == (chunks.len() - 1),
                false,
                is_gossip,
                chunk.to_vec(),
            )
        }).collect()
    }
}

impl std::fmt::Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Packet seq_num: {}", self.header.seq_num)
    }
}

#[derive(Clone)]
pub struct Header {
    pub src_addr: SocketAddr,   // 6 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub origin: SocketAddr,     // 18 bytes
    pub seq_num: u32,           // 22 bytes
    pub flags: Flags,           // 23 bytes
    pub checksum: u32,          // 27 bytes
}

// Implementação para que o cabeçalho seja conversível em bytes e vice-versa
impl Header {
    // Sempre deve-se alterar o tamanho do cabeçalho ao alterar o Header
    pub const HEADER_SIZE: usize = 27;
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr, origin: SocketAddr,
            seq_num: u32, flags: Flags, checksum: u32) -> Self {
        // Gera um timestamp usando o relógio local
        Self {
            src_addr,
            dst_addr,
            origin,
            seq_num,
            flags,
            checksum: checksum,
        }
    }

    pub fn get_ack(&self) -> Self {
        let flags = self.flags | Flags::ACK;
        Self {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            origin: self.origin,
            seq_num: self.seq_num,
            flags,
            // TODO: Fix the checksum gambiarra
            checksum: Packet::checksum(&self, &Vec::new())+1,
        }
    }

    pub fn is_last(&self) -> bool {
        self.flags.is_set(Flags::LST)
    }

    pub fn is_ack(&self) -> bool {
        self.flags.is_set(Flags::ACK)
    }

    pub fn must_gossip(&self) -> bool {
        self.flags.is_set(Flags::GSP)
    }

    fn addr_to_bytes(addr: SocketAddr) -> Vec<u8> {
        let mut bytes = Vec::new();
        match addr.ip() {
            IpAddr::V4(ipv4) => bytes.extend_from_slice(&ipv4.octets()),
            IpAddr::V6(ipv6) => bytes.extend_from_slice(&ipv6.octets()),
        }
        bytes.extend_from_slice(&addr.port().to_be_bytes());
        bytes
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&Header::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Header::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&Header::addr_to_bytes(self.origin));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.push(self.flags.value);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    fn addr_from_bytes(bytes: &[u8], start: &mut usize) -> SocketAddr {
        let ip = IpAddr::V4(Ipv4Addr::from([bytes[*start],
            bytes[*start + 1], bytes[*start + 2], bytes[*start + 3]]));
        let port = u16::from_be_bytes([bytes[*start + 4], bytes[*start + 5]]);
        *start += 6;
        SocketAddr::new(ip, port)
    }

    fn u32_from_bytes(bytes: &[u8], start: &mut usize) -> u32 {
        let out = u32::from_be_bytes([bytes[*start], bytes[*start + 1], bytes[*start + 2], bytes[*start + 3]]);
        *start += 4;
        out
    }

    pub fn from_bytes(bytes: [u8; Header::HEADER_SIZE]) -> Self {
        let mut start = 0;
        let src_addr = Header::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Header::addr_from_bytes(&bytes, &mut start);
        let origin = Header::addr_from_bytes(&bytes, &mut start);
        let seq_num = Header::u32_from_bytes(&bytes, &mut start);
        let flags = bytes[start].into();
        start += 1;
        let checksum = Header::u32_from_bytes(&bytes, &mut start);
        Header::new(
            src_addr,
            dst_addr,
            origin,
            seq_num,
            flags,
            checksum,
        )
    }
}
