// Importações necessárias
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
#[allow(dead_code)]
pub enum Header {
    SendData(HeaderSendData),
    BroadcastData(HeaderBroadcastData),
    Ack(HeaderAck),
    Syn,
    Fin,
}

#[allow(dead_code)]
impl Header {
    const SD: u8 = 0;
    const BD: u8 = 1;
    const ACK: u8 = 2;
    const SYN: u8 = 3;
    const FIN: u8 = 4; 
    fn sum_addr(addr: SocketAddr) -> u32 {
        let value = match addr.ip() {
            IpAddr::V4(ipv4) => u32::from_be_bytes(ipv4.octets()),
            IpAddr::V6(ipv6) => u128::from_be_bytes(ipv6.octets()) as u32,
        };
        value.wrapping_add(addr.port() as u32)
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
        let bytes = match self {
            Header::SendData(header) => header.to_bytes(),
            Header::BroadcastData(header) => header.to_bytes(),
            Header::Ack(header) => header.to_bytes(),
            Header::Syn => todo!("Syn header is not implemented"),
            Header::Fin => todo!("Fin header is not implemented"),            
        };
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

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        match bytes[0] {
            Header::SD => {
                let header = HeaderSendData::from_bytes(bytes[1..HeaderSendData::SIZE].to_vec());
                Header::SendData(header)
            },
            Header::BD => {
                let header = HeaderBroadcastData::from_bytes(bytes[1..HeaderBroadcastData::SIZE].to_vec());
                Header::BroadcastData(header)
            },
            Header::ACK => {
                let header = HeaderAck::from_bytes(bytes[1..HeaderAck::SIZE].to_vec());
                Header::Ack(header)
            },
            Header::SYN => todo!(),
            Header::FIN => todo!(),
            _ => panic!("Invalid header type"),            
        }
    }

    pub fn checksum(header: &Header) -> u32 {
        match header {
            Header::SendData(header) => {
                HeaderSendData::checksum(header)
            },
            Header::BroadcastData(header) => {
                HeaderBroadcastData::checksum(header)
            },
            Header::Ack(header) => {
                HeaderAck::checksum(header)
            },
            Header::Syn => todo!(),
            Header::Fin => todo!(),
        }
    }
}

pub struct HeaderSendData {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub is_last: bool,          // 17 bytes
    pub checksum: u32,          // 21 bytes
}

impl HeaderSendData {
    pub const SIZE: usize = 21;
    pub fn checksum(header: &HeaderSendData) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Header::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Header::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum = sum.wrapping_add(header.is_last as u32);
        sum
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(Header::SD);
        bytes.extend_from_slice(&Header::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Header::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.push(self.is_last as u8);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut start = 0;
        let src_addr = Header::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Header::addr_from_bytes(&bytes, &mut start);
        let seq_num = Header::u32_from_bytes(&bytes, &mut start);
        let is_last = bytes[start] != 0;
        start += 1;
        let checksum = Header::u32_from_bytes(&bytes, &mut start);
        Self {
            src_addr,
            dst_addr,
            seq_num,
            is_last,
            checksum,
        }
    }
    pub fn size() -> usize {
        // calculates the size of the struct header in bytes
        std::mem::size_of::<Self>()
    }
}

pub struct HeaderBroadcastData {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub origin: SocketAddr,     // 18 bytes
    pub seq_num: u32,           // 22 bytes
    pub is_last: bool,          // 23 bytes
    pub checksum: u32,          // 27 bytes
}

impl HeaderBroadcastData {
    pub const SIZE: usize = 27; 
    pub fn checksum(header: &HeaderBroadcastData) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Header::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Header::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(Header::sum_addr(header.origin));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum = sum.wrapping_add(header.is_last as u32);
        sum
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(Header::BD);
        bytes.extend_from_slice(&Header::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Header::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&Header::addr_to_bytes(self.origin));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.push(self.is_last as u8);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut start = 0;
        let src_addr = Header::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Header::addr_from_bytes(&bytes, &mut start);
        let origin = Header::addr_from_bytes(&bytes, &mut start);
        let seq_num = Header::u32_from_bytes(&bytes, &mut start);
        let is_last = bytes[start] != 0;        start += 1;
        let checksum = Header::u32_from_bytes(&bytes, &mut start);
        Self {
            src_addr,
            dst_addr,
            origin,
            seq_num,
            is_last,
            checksum,
        }
    }
    pub fn size() -> usize {
        // calculates the size of the struct header in bytes
        std::mem::size_of::<Self>()
    }
}

pub struct HeaderAck {
    pub src_addr: SocketAddr,   // 06 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub checksum: u32,          // 20 bytes
}

impl HeaderAck {
    pub const SIZE: usize = 20;
    pub fn checksum(header: &HeaderAck) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(Header::sum_addr(header.src_addr));
        sum = sum.wrapping_add(Header::sum_addr(header.dst_addr));
        sum = sum.wrapping_add(header.seq_num as u32);
        sum
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(Header::ACK);
        bytes.extend_from_slice(&Header::addr_to_bytes(self.src_addr));
        bytes.extend_from_slice(&Header::addr_to_bytes(self.dst_addr));
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut start = 0;
        let src_addr = Header::addr_from_bytes(&bytes, &mut start);
        let dst_addr = Header::addr_from_bytes(&bytes, &mut start);
        let seq_num = Header::u32_from_bytes(&bytes, &mut start);
        let checksum = Header::u32_from_bytes(&bytes, &mut start);
        Self {
            src_addr,
            dst_addr,
            seq_num,
            checksum,
        }
    }
    pub fn size() -> usize {
        // calculates the size of the struct header in bytes
        std::mem::size_of::<Self>()
    }
}

#[allow(dead_code)]
trait HasData {
    fn get_ack(&self) -> HeaderAck;
}

impl HasData for HeaderSendData {
    fn get_ack(&self) -> HeaderAck {
        HeaderAck {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            seq_num: self.seq_num,
            checksum: 0,
        }
    }    
}

impl HasData for HeaderBroadcastData {
    fn get_ack(&self) -> HeaderAck {
        HeaderAck {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            seq_num: self.seq_num,
            checksum: 0,
        }
    }    
    
}
