// Importações necessárias
use std::net::{SocketAddr, IpAddr, Ipv4Addr, Ipv6Addr};

// Tamanho do buffer
use crate::config::BUFFER_SIZE;

// sempre deve-se alterar o tamanho do cabeçalho se alterar o Header
pub const HEADER_SIZE: usize = 32; // Header::new_empty().to_bytes().len()
// estrutura para o cabeçalho
pub struct Header {
    pub src_addr: SocketAddr,
    pub dst_addr: SocketAddr,
    pub ack_num: u32,
    pub seq_num: u32,
    pub msg_size: usize,
    pub checksum: u16,
    pub flags: u8,
    pub is_last: bool,
    // um vetor mutável de bytes
    pub msg: Vec<u8>,
}
// implementação para que o cabeçalho seja conversível em bytes e vice-versa
impl<'a> Header {
    pub fn new(
        src_addr: SocketAddr, dst_addr: SocketAddr,
        ack_num: u32, seq_num: u32, msg_size: usize,
        flags: u8,
        is_last: bool,
        msg: Vec<u8>
    ) -> Self {
        let checksum = Header::get_checksum(&msg);
        Self {
            src_addr,
            dst_addr,
            ack_num,
            seq_num,
            msg_size,
            checksum,
            flags,
            is_last,
            msg,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            src_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            dst_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            ack_num: 0,
            seq_num: 0,
            msg_size: 0,
            checksum: 0,
            flags: 0,
            is_last: false,
            // a mensagem é uma array de bytes vazio
            msg: Vec::new(),
        }
    }

    pub fn get_ack(&self) -> Self {
        Self {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            ack_num: self.seq_num,
            seq_num: 0,
            msg_size: 0,
            checksum: 0,
            flags: 1,
            is_last: false,
            msg: Vec::new(),
        }
    }

    pub fn is_ack(&self) -> bool {
        self.flags & 1 == 1
    }

    pub fn get_checksum(msg: &Vec<u8>) -> u16 {
        let mut sum: u16 = 0;
        for byte in msg {
            // adds without overflow
            sum = sum.wrapping_add(*byte as u16);
        }
        sum as u16
    }

    pub fn clone (&self) -> Self {
        Self {
            src_addr: self.src_addr,
            dst_addr: self.dst_addr,
            ack_num: self.ack_num,
            seq_num: self.seq_num,
            msg_size: self.msg_size,
            checksum: self.checksum,
            flags: self.flags,
            is_last: self.is_last,
            msg: self.msg.clone(),
        }
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
        bytes.extend_from_slice(&self.ack_num.to_be_bytes());
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.extend_from_slice(&self.msg_size.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.push(self.flags);
        bytes.push(self.is_last as u8);
        bytes.extend_from_slice(self.msg.as_slice());
        bytes
    }

    pub fn from_bytes(&mut self, mut bytes: [u8; BUFFER_SIZE]){
        let src_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from([bytes[0], bytes[1], bytes[2], bytes[3]])),
            u16::from_be_bytes([bytes[4], bytes[5]]),
        );
        let dst_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from([bytes[6], bytes[7], bytes[8], bytes[9]])),
            u16::from_be_bytes([bytes[10], bytes[11]]),
        );
        let ack_num = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let seq_num = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let msg_size = usize::from_be_bytes([
            bytes[20], bytes[21], bytes[22], bytes[23], 
            bytes[24], bytes[25], bytes[26], bytes[27]
        ]);
        let checksum = u16::from_be_bytes([bytes[28], bytes[29]]);
        let flags = bytes[30];
        let is_last = bytes[31] == 1;
        let mut msg = bytes[HEADER_SIZE..].to_vec();
        
        self.src_addr = src_addr;
        self.dst_addr = dst_addr;
        self.ack_num = ack_num;
        self.seq_num = seq_num;
        self.msg_size = msg_size;
        self.checksum = checksum;
        self.flags = flags;
        self.is_last = is_last;
        self.msg = msg;
    }

    pub fn create_from_bytes(bytes: [u8; BUFFER_SIZE]) -> Self {
        Header::new(
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from([bytes[0], bytes[1], bytes[2], bytes[3]])),
                u16::from_be_bytes([bytes[4], bytes[5]]),
            ),
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from([bytes[6], bytes[7], bytes[8], bytes[9]])),
                u16::from_be_bytes([bytes[10], bytes[11]])),
            u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            usize::from_be_bytes([
                bytes[20], bytes[21], bytes[22], bytes[23], 
                bytes[24], bytes[25], bytes[26], bytes[27]
            ]),
            bytes[30],
            bytes[31] == 1,
            bytes[HEADER_SIZE..].to_vec(),
        )
    }
}
