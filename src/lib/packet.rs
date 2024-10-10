// Importações necessárias
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

// Tamanho do buffer
use crate::config::{CORRUPTION_RATE, BUFFER_SIZE};
use super::flags::Flags;

#[derive(Clone)]
pub struct Packet {
    pub header: Header,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr, seq_num: u32,
            checksum: Option<u16>, is_last: bool, is_ack: bool, is_dlv: bool,
            data: Vec<u8>) -> Self {
        let mut flags = Flags::EMP;
        if is_last {
            flags = flags | Flags::LST;
        }
        if is_ack {
            flags = flags | Flags::ACK;
        }
        if is_dlv {
            flags = flags | Flags::DLV;
        }
        let checksum = checksum.or_else(|| Some(Self::checksum(&Header::new(src_addr, dst_addr, seq_num, flags, None), &data)));
        let header = Header::new(src_addr, dst_addr, seq_num, flags, checksum);
        Self { header, data }
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

    pub fn from_bytes(bytes: [u8; BUFFER_SIZE], data_size: usize) -> Self {
        let header = Header::from_bytes(bytes[..HEADER_SIZE].try_into().unwrap());
        let data = bytes[HEADER_SIZE..data_size].to_vec();
        Self { header, data }
    }

    pub fn checksum(header: &Header, data: &Vec<u8>) -> u16 {
        let mut sum: u32 = 0;
        match header.src_addr.ip() {
            IpAddr::V4(ipv4) => sum = sum.wrapping_add(u32::from_be_bytes(ipv4.octets())),
            IpAddr::V6(ipv6) => sum = sum.wrapping_add(u128::from_be_bytes(ipv6.octets()) as u32),
        }
        match header.dst_addr.ip() {
            IpAddr::V4(ipv4) => sum = sum.wrapping_add(u32::from_be_bytes(ipv4.octets())),
            IpAddr::V6(ipv6) => sum = sum.wrapping_add(u128::from_be_bytes(ipv6.octets()) as u32),
        }
        sum = sum.wrapping_add(header.src_addr.port() as u32);
        sum = sum.wrapping_add(header.dst_addr.port() as u32);
        sum = sum.wrapping_add(header.seq_num as u32);
        sum = sum.wrapping_add(header.flags.value as u32);
        for byte in data {
            sum = sum.wrapping_add(*byte as u32);
        }
        if rand::random::<f32>() < CORRUPTION_RATE {
            sum = sum.wrapping_add(1);
        }
        sum as u16
    }
}

#[derive(Clone)]
pub struct Header {
    pub src_addr: SocketAddr,   // 6 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub flags: Flags,  // ack: 1, last: 2, syn: 4, fin: 8   // 17 bytes
    pub checksum: u16,          // 19 bytes
}

// Sempre deve-se alterar o tamanho do cabeçalho ao alterar o Header
pub const HEADER_SIZE: usize = 19;

// Implementação para que o cabeçalho seja conversível em bytes e vice-versa
impl Header {
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr, seq_num: u32, flags: Flags, checksum: Option<u16>) -> Self {
        Self {
            src_addr,
            dst_addr,
            seq_num,
            flags,
            checksum: checksum.unwrap_or(0),
        }
    }

    pub fn get_ack(&self) -> Self {
        let flags = self.flags | Flags::ACK;
        Self {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
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
        self.flags.is_set(Flags::ACK) && !self.flags.is_set(Flags::DLV)
    }

    pub fn is_dlv(&self) -> bool {
        self.flags.is_set(Flags::DLV)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self.src_addr.ip() {
            IpAddr::V4(ipv4) => bytes.extend_from_slice(&ipv4.octets()),
            IpAddr::V6(ipv6) => bytes.extend_from_slice(&ipv6.octets()),
        }
        bytes.extend_from_slice(&self.src_addr.port().to_be_bytes());
        match self.dst_addr.ip() {
            IpAddr::V4(ipv4) => bytes.extend_from_slice(&ipv4.octets()),
            IpAddr::V6(ipv6) => bytes.extend_from_slice(&ipv6.octets()),
        }
        bytes.extend_from_slice(&self.dst_addr.port().to_be_bytes());
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.push(self.flags.value);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: [u8; HEADER_SIZE]) -> Self {
        Header::new(
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from([bytes[0], bytes[1], bytes[2], bytes[3]])),
                u16::from_be_bytes([bytes[4], bytes[5]]),
            ),
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from([bytes[6], bytes[7], bytes[8], bytes[9]])),
                u16::from_be_bytes([bytes[10], bytes[11]]),
            ),
            u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            bytes[16].into(),
            Some(u16::from_be_bytes([bytes[17], bytes[18]])),
        )
    }
}
