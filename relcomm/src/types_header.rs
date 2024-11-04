// Importações necessárias
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Funções auxiliares e as obrigatoriamente implementadas
pub trait Header {
    const SD: u8 = 0;
    const BD: u8 = 1;
    const ACK: u8 = 2;
    const LREQ: u8 = 3;
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

#[derive(Clone)]
pub struct HSnd {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub is_last: bool,          // 17 bytes
    pub checksum: u32,          // 21 bytes
}

impl HSnd {
    const SIZE: usize = 21;
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

#[derive(Clone)]
pub struct HBrd {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub origin: SocketAddr,     // 18 bytes
    pub seq_num: u32,           // 22 bytes
    pub is_last: bool,          // 23 bytes
    pub checksum: u32,          // 27 bytes
}

impl HBrd {
    pub const SIZE: usize = 27;
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

#[derive(Clone)]
pub struct Ack {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub checksum: u32,          // 20 bytes
}

impl Ack {
    pub const SIZE: usize = 20;    
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

#[derive(Clone)]
pub struct HLRq {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub is_last: bool,          // 17 bytes
    pub checksum: u32,          // 21 bytes
}

impl HLRq {
    const SIZE: usize = 21;
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

pub trait DataHeader: Header {
    fn new(addr: &Vec<SocketAddr>, seq_num: u32, is_last: bool) -> Self;
    fn get_ack(&self) -> Ack;
}

impl DataHeader for HSnd {
    fn new(addr: &Vec<SocketAddr>, seq_num: u32, is_last: bool) -> Self {
        Self {
            src_addr: addr[0],
            dst_addr: addr[1],
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
}

impl DataHeader for HBrd {
    fn new(addr: &Vec<SocketAddr>, seq_num: u32, is_last: bool) -> Self {
        Self {
            src_addr: addr[0],
            dst_addr: addr[1],
            origin: addr[2],
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
}

impl DataHeader for HLRq {
    fn new(addr: &Vec<SocketAddr>, seq_num: u32, is_last: bool) -> Self {
        Self {
            src_addr: addr[0],
            dst_addr: addr[1],
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
}

/*
#[derive(Clone)]
pub enum Header {
    Send(HSnd),
    Broadcast(HBrd),
    /// because ACK should be built only from another header
    Ack(Ack),
}
impl Header {
    
    pub fn get_dst_addr(&self) -> SocketAddr {
        match self {
            Header::Send(header) => header.dst_addr,
            Header::Broadcast(header) => header.dst_addr,
            Header::Ack(header) => header.dst_addr,
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes = match self {
            Header::Send(header) => header.to_bytes(),
            Header::Broadcast(header) => header.to_bytes(),
            Header::Ack(header) => header.to_bytes(),        
        };
        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        match bytes[0] {
            Header::SD => {
                let header = HSnd::from_bytes(bytes[1..HSnd::size()].to_vec());
                Header::Send(header)
            },
            Header::BD => {
                let header = HBrd::from_bytes(bytes[1..HBrd::size()].to_vec());
                Header::Broadcast(header)
            },
            Header::ACK => {
                let header = Ack::from_bytes(bytes[1..Ack::size()].to_vec());
                Header::Ack(header)
            },
            _ => panic!("Invalid header type"),            
        }
    }
    pub fn checksum(header: &Header) -> u32 {
        match header {
            Header::Send(header) => {
                HSnd::checksum(header)
            },
            Header::Broadcast(header) => {
                HBrd::checksum(header)
            },
            Header::Ack(header) => {
                Ack::checksum(header)
            },
        }
    }
    pub fn size(&self) -> usize {
        match self {
            Header::Send(_) => HSnd::size(),
            Header::Broadcast(_) => HBrd::size(),
            Header::Ack(_) => Ack::size(),
        }
    }
}
*/
