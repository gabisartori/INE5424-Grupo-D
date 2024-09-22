// Importações necessárias
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

// Tamanho do buffer
use crate::config::BUFFER_SIZE;

// sempre deve-se alterar o tamanho do cabeçalho se alterar o Header
pub const HEADER_SIZE: usize = 19; // Header::new_empty().to_bytes().len()
// estrutura para o cabeçalho

#[derive(Clone)]
pub struct Packet {
    pub header: Header,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr, seq_num: u32, flags: u8, checksum: Option<u16>, data: Vec<u8>) -> Self {
        let checksum = checksum.or_else(|| Some(Self::checksum(&Header::new(src_addr, dst_addr, seq_num, flags, None), &data)));
        let header = Header::new(src_addr, dst_addr, seq_num, flags, checksum);
        Self { header, data }
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
            IpAddr::V4(ipv4) => sum += u32::from_be_bytes(ipv4.octets()),
            IpAddr::V6(ipv6) => sum += u128::from_be_bytes(ipv6.octets()) as u32,
        }
        sum += header.src_addr.port() as u32;
        match header.dst_addr.ip() {
            IpAddr::V4(ipv4) => sum += u32::from_be_bytes(ipv4.octets()),
            IpAddr::V6(ipv6) => sum += u128::from_be_bytes(ipv6.octets()) as u32,
        }
        sum += header.dst_addr.port() as u32;
        sum += header.seq_num as u32;
        sum += header.flags as u32;
        for byte in data {
            sum += *byte as u32;
        }
        sum as u16
    }

    pub fn is_ack(&self) -> bool {
        self.header.is_ack()
    }

    pub fn is_last(&self) -> bool {
        self.header.is_last()
    }
}

#[derive(Clone)]
pub struct Header {
    pub src_addr: SocketAddr,   // 6 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub flags: u8,  // ack: 1, last: 2, syn: 4, fin: 8   // 17 bytes
    pub checksum: u16,          // 19 bytes
}
// implementação para que o cabeçalho seja conversível em bytes e vice-versa
impl<'a> Header {
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr, seq_num: u32, flags: u8, checksum: Option<u16>) -> Self {
        Self {
            src_addr,
            dst_addr,
            seq_num,
            flags,
            checksum: checksum.unwrap_or(0),
        }
    }

    pub fn get_ack(&self) -> Self {
        let flags = self.flags | 1;
        Self {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            seq_num: self.seq_num,
            flags: flags,
            checksum: 0,
        }
    }

    pub fn is_ack(&self) -> bool {
        (self.flags & 1) == 1
    }

    pub fn is_last(&self) -> bool {
        (self.flags & 2) == 2
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
        bytes.push(self.flags);
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
            u8::from_be_bytes([bytes[16]]),
            Some(u16::from_be_bytes([bytes[17], bytes[18]])),
        )
    }
}
