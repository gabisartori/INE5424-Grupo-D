// Importações necessárias
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Clone)]
pub struct HSnd {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub is_last: bool,          // 17 bytes
    pub checksum: u32,          // 21 bytes
}

#[derive(Clone)]
pub struct HBrd {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub origin: SocketAddr,     // 18 bytes
    pub seq_num: u32,           // 22 bytes
    pub is_last: bool,          // 23 bytes
    pub checksum: u32,          // 27 bytes
}

#[derive(Clone)]
pub struct HLRq {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub is_last: bool,          // 17 bytes
    pub checksum: u32,          // 21 bytes
}

#[derive(Clone)]
pub struct Ack {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub checksum: u32,          // 20 bytes
}

#[derive(Clone)]
pub struct AckBrd {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub origin: SocketAddr,     // 18 bytes
    pub seq_num: u32,           // 22 bytes
    pub checksum: u32,          // 26 bytes
}

impl HSnd {
    pub const SIZE: usize = 21;
    pub const TYPE: &'static str = "Send";
}

impl HBrd {
    pub const SIZE: usize = 27;
    pub const TYPE: &'static str = "Broadcast";
}

impl HLRq {
    pub const SIZE: usize = 21;
    pub const TYPE: &'static str = "LeaderRequest";
}

impl Ack {
    pub const SIZE: usize = 20;
    pub const TYPE: &'static str = "Ack";    
}

impl AckBrd {
    pub const SIZE: usize = 26;
    pub const TYPE: &'static str = "AckBroadcast";
}

/// Funções auxiliares e as obrigatoriamente implementadas
pub trait Header {
    const SD: u8 = 0;
    const BD: u8 = 1;
    const ACK: u8 = 2;
    const ACK_BD: u8 = 3;
    const LREQ: u8 = 4;
    fn sum_addr(addr: SocketAddr) -> u32 {
        let value = match addr.ip() {
            IpAddr::V4(ipv4) => u32::from_be_bytes(ipv4.octets()),
            IpAddr::V6(ipv6) => u128::from_be_bytes(ipv6.octets()) as u32,
        };
        value.wrapping_add(addr.port() as u32)
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

    fn addr_to_bytes(addr: SocketAddr) -> Vec<u8> {
        let mut bytes = Vec::new();
        match addr.ip() {
            IpAddr::V4(ipv4) => bytes.extend_from_slice(&ipv4.octets()),
            IpAddr::V6(ipv6) => bytes.extend_from_slice(&ipv6.octets()),
        }
        bytes.extend_from_slice(&addr.port().to_be_bytes());
        bytes
    }
    
    fn checksum(header: &Self) -> u32;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: Vec<u8>) -> Self;
    fn size() -> usize;
}

impl Header for HSnd {
    fn checksum(header: &HSnd) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Self::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum = sum.wrapping_add(header.is_last as u32);
        sum
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(Self::SD);
        bytes.extend_from_slice(&Self::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Self::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.push(self.is_last as u8);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut start = 0;
        let src_addr = Self::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Self::addr_from_bytes(&bytes, &mut start);
        let seq_num = Self::u32_from_bytes(&bytes, &mut start);
        let is_last = bytes[start] != 0;
        start += 1;
        let checksum = Self::u32_from_bytes(&bytes, &mut start);
        Self {
            src_addr,
            dst_addr,
            seq_num,
            is_last,
            checksum,
        }
    }

    fn size() -> usize {
        // calculates the size of the struct header in bytes
        Self::SIZE
    }
}

impl Header for HBrd {
    fn checksum(header: &HBrd) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Self::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.origin));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum = sum.wrapping_add(header.is_last as u32);
        sum
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(Self::BD);
        bytes.extend_from_slice(&Self::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Self::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&Self::addr_to_bytes(self.origin));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.push(self.is_last as u8);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut start = 0;
        let src_addr = Self::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Self::addr_from_bytes(&bytes, &mut start);
        let origin = Self::addr_from_bytes(&bytes, &mut start);
        let seq_num = Self::u32_from_bytes(&bytes, &mut start);
        let is_last = bytes[start] != 0;        start += 1;
        let checksum = Self::u32_from_bytes(&bytes, &mut start);
        Self {
            src_addr,
            dst_addr,
            origin,
            seq_num,
            is_last,
            checksum,
        }
    }
    
    fn size() -> usize {
        // calculates the size of the struct header in bytes
        Self::SIZE
    }

}

impl Header for Ack {
    fn checksum(header: &Ack) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Self::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(Self::ACK);
        bytes.extend_from_slice(&Self::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Self::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut start = 0;
        let src_addr = Self::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Self::addr_from_bytes(&bytes, &mut start);
        let seq_num = Self::u32_from_bytes(&bytes, &mut start);
        let checksum = Self::u32_from_bytes(&bytes, &mut start);
        Self {
            src_addr,
            dst_addr,
            seq_num,
            checksum,
        }
    }

    fn size() -> usize {
        // calculates the size of the struct header in bytes
        Self::SIZE
    }
}

impl Header for HLRq {
    fn checksum(header: &HLRq) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Self::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum = sum.wrapping_add(header.is_last as u32);
        sum
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(Self::LREQ);
        bytes.extend_from_slice(&Self::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Self::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.push(self.is_last as u8);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut start = 0;
        let src_addr = Self::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Self::addr_from_bytes(&bytes, &mut start);
        let seq_num = Self::u32_from_bytes(&bytes, &mut start);
        let is_last = bytes[start] != 0;
        start += 1;
        let checksum = Self::u32_from_bytes(&bytes, &mut start);
        Self {
            src_addr,
            dst_addr,
            seq_num,
            is_last,
            checksum,
        }
    }

    fn size() -> usize {
        // calculates the size of the struct header in bytes
        Self::SIZE
    }
}

impl Header for AckBrd {
    fn checksum(header: &AckBrd) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Self::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(Self::sum_addr(header.origin));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(Self::ACK_BD);
        bytes.extend_from_slice(&Self::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Self::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&Self::addr_to_bytes(self.origin));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut start = 0;
        let src_addr = Self::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Self::addr_from_bytes(&bytes, &mut start);
        let origin = Self::addr_from_bytes(&bytes, &mut start);
        let seq_num = Self::u32_from_bytes(&bytes, &mut start);
        let checksum = Self::u32_from_bytes(&bytes, &mut start);
        Self {
            src_addr,
            dst_addr,
            origin,
            seq_num,
            checksum,
        }
    }

    fn size() -> usize {
        // calculates the size of the struct header in bytes
        Self::SIZE
    }
    
}

pub trait IsAck: Clone + Header {}
impl IsAck for Ack {}
impl IsAck for AckBrd {}

pub trait DataHeader: Header {
    type AckType: IsAck;
    fn new(addr: &Vec<&SocketAddr>, seq_num: u32, is_last: bool) -> Self;
    fn get_ack(&self) -> Self::AckType;
    fn is_last(&self) -> bool;
}

impl DataHeader for HSnd {
    type AckType = Ack;
    fn new(addr: &Vec<&SocketAddr>, seq_num: u32, is_last: bool) -> Self {
        Self {
            src_addr: *addr[0],
            dst_addr: *addr[1],
            seq_num,
            is_last,
            checksum: 0,
        }
    }
    fn get_ack(&self) -> Ack {
        let mut ack = Ack {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            seq_num: self.seq_num,
            checksum: 0,
        };
        ack.checksum = Ack::checksum(&ack);
        ack
    }
    fn is_last(&self) -> bool {
        self.is_last
    }
}

impl DataHeader for HBrd {
    type AckType = AckBrd;
    fn new(addr: &Vec<&SocketAddr>, seq_num: u32, is_last: bool) -> Self {
        Self {
            src_addr: *addr[0],
            dst_addr: *addr[1],
            origin: *addr[2],
            seq_num,
            is_last,
            checksum: 0,
        }
    }

    fn get_ack(&self) -> AckBrd {
        let mut ack = AckBrd {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            origin: self.origin,
            seq_num: self.seq_num,
            checksum: 0,
        };
        ack.checksum = AckBrd::checksum(&ack);
        ack
    }

    fn is_last(&self) -> bool {
        self.is_last
    }
}

impl DataHeader for HLRq {
    type AckType = Ack;
    fn new(addr: &Vec<&SocketAddr>, seq_num: u32, is_last: bool) -> Self {
        Self {
            src_addr: *addr[0],
            dst_addr: *addr[1],
            seq_num,
            is_last,
            checksum: 0,
        }
    }

    fn get_ack(&self) -> Ack {
        let mut ack = Ack {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            seq_num: self.seq_num,
            checksum: 0,
        };
        ack.checksum = Ack::checksum(&ack);
        ack
    }

    fn is_last(&self) -> bool {
        self.is_last
    }
}
