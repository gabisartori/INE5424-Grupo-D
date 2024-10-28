// Importações necessárias
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use crate::flags::Flags;
// Ignore unused functions while handshake isn't implemented
#[derive(Clone)]
pub struct Header {
    pub src_addr: SocketAddr,   // 6 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub origin: SocketAddr,     // 18 bytes
    pub seq_num: u32,           // 22 bytes
    pub flags: Flags,           // 23 bytes
    pub checksum: u32,          // 27 bytes
}

#[allow(dead_code)]
// Implementação para que o cabeçalho seja conversível em bytes e vice-versa
impl Header {
    // Sempre deve-se alterar o tamanho do cabeçalho ao alterar o Header
    pub const HEADER_SIZE: usize = 27;
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr, origin: SocketAddr,
            seq_num: u32, flags: Flags, checksum: u32) -> Self {
        Self {
            src_addr,
            dst_addr,
            origin,
            seq_num,
            flags,
            checksum,
        }
    }

    pub fn get_ack(&self) -> Self {
        let flags = self.flags | Flags::ACK;
        let mut ack = Self {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            origin: self.origin,
            seq_num: self.seq_num,
            flags,
            checksum: 0,
        };
        ack.checksum = Self::checksum(&ack);
        ack
    }

    fn sum_addr(addr: SocketAddr) -> u32 {
        let value = match addr.ip() {
            IpAddr::V4(ipv4) => u32::from_be_bytes(ipv4.octets()),
            IpAddr::V6(ipv6) => u128::from_be_bytes(ipv6.octets()) as u32,
        };
        value.wrapping_add(addr.port() as u32)
    }

    pub fn checksum(header: &Header) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Header::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Header::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(Header::sum_addr(header.origin));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum = sum.wrapping_add(header.flags.value as u32);
        sum
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

    pub fn is_syn(&self) -> bool {
        self.flags.is_set(Flags::SYN)
    }

    pub fn is_fin(&self) -> bool {
        self.flags.is_set(Flags::FIN)
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
