#![allow(dead_code)]
// Importações necessárias
use std::net::SocketAddr;
use std::array::TryFromSliceError;
use crate::types_header::{HeaderSendData, HeaderBroadcastData, HeaderAck, Header};
pub const BUFFER_SIZE: usize = 2<<9;
struct SendData {
    data: Vec<u8>,
}

struct BroadcastData {
    data: Vec<u8>,
}

struct Ack {}

trait IsPacket: Sized {
    // TODO: research how inheritance actually works in Rust
    fn checksum(&self) -> u32;
    fn size() -> usize;
    fn from_bytes(bytes: Vec<u8>, data_size: usize) -> Vec<u8>;
    fn to_bytes(&self) -> Vec<u8>;
}
impl IsPacket for SendData {
    fn checksum(&self) -> u32 {
        self.data.iter().fold(0, |acc, &byte| acc.wrapping_add(byte as u32))
    }
    fn size() -> usize {
        HeaderSendData::size()
    }
    fn from_bytes(bytes: Vec<u8>, data_size: usize) -> Vec<u8> {
        bytes[HeaderSendData::size()..data_size].to_vec()
    }
    fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}
impl IsPacket for BroadcastData {
    fn checksum(&self) -> u32 {
        self.data.iter().fold(0, |acc, &byte| acc.wrapping_add(byte as u32))
    }
    fn size() -> usize {
        HeaderBroadcastData::size()
    }
    fn from_bytes(bytes: Vec<u8>, data_size: usize) -> Vec<u8> {
        bytes[HeaderBroadcastData::size()..data_size].to_vec()
    }
    fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}
impl IsPacket for Ack {
    fn checksum(&self) -> u32 {
        0
    }
    fn size() -> usize {
        HeaderAck::size()
    }
    fn from_bytes(_: Vec<u8>, _data_size: usize) -> Vec<u8> {
        Vec::new()
    }
    fn to_bytes(&self) -> Vec<u8> {
        Vec::new()
    }
}

trait HasData {}
impl HasData for SendData {}
impl HasData for BroadcastData {}

struct Packet<IsPacket> {
    pkt: IsPacket,
    header: Header,
}

impl<Pkt> Packet<Pkt> where Pkt: IsPacket {
    pub const BUFFER_SIZE: usize = 2<<9;
    pub fn checksum(&self) -> u32 {
        self.pkt.checksum() + Header::checksum(&self.header)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.pkt.to_bytes());
        bytes
    }
}

impl<T> Packet<T> where T: HasData, T: IsPacket {}

impl Packet<SendData> {
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr,
            seq_num: u32, is_last: bool, data: Vec<u8>) -> Self {
        let mut header = HeaderSendData {
            src_addr,
            dst_addr,
            seq_num,
            is_last,
            checksum: 0,
        };
        header.checksum = HeaderSendData::checksum(&header);
        let header = Header::SendData(header);
        let pkt = SendData { data };
        Self { pkt, header }
    }

    fn packets_from_message(
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        seq_num: u32,
        data: Vec<u8>,
    ) -> Vec<Self> {
        let chunks: Vec<&[u8]> = data.chunks(BUFFER_SIZE - HeaderBroadcastData::size()).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            Packet::<SendData>::new(
                src_addr,
                dst_addr,
                seq_num + i as u32,
                i == (chunks.len() - 1),
                chunk.to_vec(),
            )
        }).collect()
    }

    pub fn from_bytes(bytes: Vec<u8>, data_size: usize) -> Result<Self, TryFromSliceError> {
        let header = Header::from_bytes(bytes[..HeaderSendData::size()].try_into()?);
        let data = SendData::from_bytes(bytes, data_size);
        let pkt = SendData { data };
        Ok(Self { pkt, header })
    }
}

impl Packet<BroadcastData> {
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr,
            origin: SocketAddr, seq_num: u32, is_last: bool,
            data: Vec<u8>) -> Self {
        let mut header = HeaderBroadcastData {
            src_addr,
            dst_addr,
            origin,
            seq_num,
            is_last,
            checksum: 0,
        };
        header.checksum = HeaderBroadcastData::checksum(&header);
        let header = Header::BroadcastData(header);
        let pkt = BroadcastData { data };
        Self { pkt, header }
    }

    fn packets_from_message(
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        origin: SocketAddr,
        seq_num: u32,
        data: Vec<u8>,
    ) -> Vec<Self> {
        let chunks: Vec<&[u8]> = data.chunks(BUFFER_SIZE - HeaderBroadcastData::size()).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            Packet::<BroadcastData>::new(
                src_addr,
                dst_addr,
                origin,
                seq_num + i as u32,
                i == (chunks.len() - 1),
                chunk.to_vec(),
            )
        }).collect()
    }

    pub fn from_bytes(bytes: Vec<u8>, data_size: usize) -> Result<Self, TryFromSliceError> {
        let header = Header::from_bytes(bytes.clone());
        let data = BroadcastData::from_bytes(bytes, data_size);
        let pkt = BroadcastData { data };
        Ok(Self { pkt, header })
    }
}

impl Packet<Ack> {
    pub fn new(src_addr: SocketAddr,
            dst_addr: SocketAddr,
            seq_num: u32) -> Self {
        let mut header = HeaderAck {
            src_addr,
            dst_addr,
            seq_num,
            checksum: 0,
        };
        header.checksum = HeaderAck::checksum(&header);
        let header = Header::Ack(header);
        let pkt = Ack {};
        Self { pkt, header }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, TryFromSliceError> {
        let header = Header::from_bytes(bytes[..HeaderAck::size()].try_into()?);
        Ok(Self { pkt: Ack {}, header })
    }
}
