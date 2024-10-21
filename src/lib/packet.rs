// Importações necessárias
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

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
            checksum: Option<u32>, is_last: bool, is_ack: bool,is_dlv: bool,
            req_dlv: bool, data: Vec<u8>) -> Self {
        let mut flags = Flags::EMP;
        let mut timestamp = None;
        
        if is_last {
            flags = flags | Flags::LST;
            if req_dlv {
                timestamp = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap());
            }
        }
        if is_ack {
            flags = flags | Flags::ACK;
        }
        if is_dlv {
            flags = flags | Flags::DLV;
        }
        if req_dlv {
            flags = flags | Flags::RDLV;
        }
        let checksum = checksum.or_else(|| Some(Self::checksum(&Header::new(src_addr, dst_addr, seq_num, flags, None, timestamp), &data)));
        let header = Header::new(src_addr, dst_addr, seq_num, flags, checksum, timestamp);
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

    pub fn checksum(header: &Header, data: &Vec<u8>) -> u32 {
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
        sum
    }
}

#[derive(Clone)]
pub struct Header {
    pub src_addr: SocketAddr,   // 6 bytes
    pub dst_addr: SocketAddr,   // 12 bytes
    pub seq_num: u32,           // 16 bytes
    pub flags: Flags,  // ack: 1, last: 2, syn: 4, fin: 8, dlv: 16, rdlv: 32  // 17 bytes
    pub checksum: u32,          // 21 bytes
    pub timestamp: Option<Duration>,    // 33 bytes
}

// Sempre deve-se alterar o tamanho do cabeçalho ao alterar o Header
pub const HEADER_SIZE: usize = 33;

// Implementação para que o cabeçalho seja conversível em bytes e vice-versa
impl Header {
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr,
            seq_num: u32, flags: Flags, checksum: Option<u32>,
            timestamp: Option<Duration>) -> Self {
        // Gera um timestamp usando o relógio local
        Self {
            src_addr,
            dst_addr,
            seq_num,
            flags,
            checksum: checksum.unwrap_or(0),
            timestamp,
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
            timestamp: self.timestamp,
        }
    }

    pub fn is_last(&self) -> bool {
        self.flags.is_set(Flags::LST)
    }

    pub fn is_ack(&self) -> bool {
        self.flags.is_set(Flags::ACK)
    }

    // pub fn is_dlv(&self) -> bool {
    //     self.flags.is_set(Flags::DLV)
    // }

    // pub fn r_dlv(&self) -> bool {
    //     self.flags.is_set(Flags::RDLV)
    // }

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
        bytes.extend_from_slice(&Header::timestamp_to_bytes(self.timestamp));
        bytes
    }

    pub fn from_bytes(bytes: [u8; HEADER_SIZE]) -> Self {
        let mut start = 0;
        let src_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from([bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3]])),
            u16::from_be_bytes([bytes[start + 4], bytes[start + 5]]),
        );
        start += 6;
        let dst_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from([bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3]])),
            u16::from_be_bytes([bytes[start + 4], bytes[start + 5]]),
        );
        start += 6;
        let seq_num = u32::from_be_bytes([bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3]]);
        start += 4;
        let flags = bytes[start].into();
        start += 1;
        let checksum = Some(u32::from_be_bytes([bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3]]));
        start += 4;
        let timestamp = Header::bytes_to_timestamp(bytes[start..].try_into().unwrap());
        Header::new(
            src_addr,
            dst_addr,
            seq_num,
            flags,
            checksum,
            timestamp,
        )
    }

    fn timestamp_to_bytes(time_stamp: Option<Duration>) -> [u8; 12] {
        if time_stamp.is_none() {
            return [0u8; 12];
        }
        let time_stamp = time_stamp.unwrap();
        let secs = time_stamp.as_secs();
        let nanos = time_stamp.subsec_nanos();

        let mut bytes = [0u8; 12];
        bytes[..8].copy_from_slice(&secs.to_be_bytes());
        bytes[8..].copy_from_slice(&nanos.to_be_bytes());
        bytes
    }

    fn bytes_to_timestamp(bytes: [u8; 12]) -> Option<Duration> {
        if bytes.iter().all(|&b| b == 0) {
            return None;
        }
        let secs = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        let nanos = u32::from_be_bytes(bytes[8..].try_into().unwrap());
        Some((UNIX_EPOCH + Duration::new(secs, nanos)).duration_since(UNIX_EPOCH).unwrap())
    }
}
