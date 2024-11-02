// Importações necessárias
use std::net::SocketAddr;
use crate::types_header::{Header, HeaderAck, HeaderBroadcast, HeaderSend};

#[derive(Clone)]
pub enum Packet {
    Send{
        header: HeaderSend,
        data: Vec<u8>,
    },
    Broadcast{
        header: HeaderBroadcast,
        data: Vec<u8>,
    },
    Ack{
        header: HeaderAck,
    },
}

impl Packet {
    pub const BUFFER_SIZE: usize = 2<<9;
    fn new(header: Header, data: Vec<u8>) -> Self {
        let mut pkt = match header {
            Header::Send(header) => {
                Packet::Send {header, data}
            },
            Header::Broadcast(header) => {
                Packet::Broadcast {header, data}
            },
            Header::Ack(_) => { panic!("Ack packet must not have data") },
        };
        pkt.set_checksum(Packet::checksum(&pkt));
        pkt
    }
    pub fn checksum(pkt: &Packet) -> u32 {
        match pkt {
            Packet::Send{header, data} => {
                let sum: u32 = data.iter().fold(0, |acc, &byte| acc.wrapping_add(byte as u32));
                sum.wrapping_add(HeaderSend::checksum(header))
            },
            Packet::Broadcast{header, data} => {
                let sum: u32 = data.iter().fold(0, |acc, &byte| acc.wrapping_add(byte as u32));
                sum.wrapping_add(HeaderBroadcast::checksum(header))
            },
            Packet::Ack {header} => {
                HeaderAck::checksum(header)
            }
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let type_h = bytes[0];
        let bytes = bytes[1..].to_vec();
        match type_h {
            Header::SD => {
                let data = bytes[HeaderSend::size()..].to_vec();
                let header = HeaderSend::from_bytes(bytes);
                Packet::Send {header, data}
            },
            Header::BD => {
                let data = bytes[HeaderBroadcast::size()..].to_vec();
                let header = HeaderBroadcast::from_bytes(bytes);
                Packet::Broadcast {header, data}
            },
            Header::ACK => {
                let header = HeaderAck::from_bytes(bytes);
                Packet::Ack {header}
            },
            _ => panic!("Invalid packet type"),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Packet::Send {header, data} => {
                let mut bytes = header.to_bytes();
                bytes.extend_from_slice(&data);
                bytes
            },
            Packet::Broadcast {header, data} => {
                let mut bytes = header.to_bytes();
                bytes.extend_from_slice(&data);
                bytes
            },
            Packet::Ack {header} => {
                header.to_bytes()
            }
        }
    }
}

impl Packet {
    pub fn get_checksum(&self) -> u32 {
        match self {
            Packet::Send {header, ..} => header.checksum,
            Packet::Broadcast {header, ..} => header.checksum,
            Packet::Ack {header, ..} => header.checksum,
        }
    }
    pub fn set_checksum(&mut self, value: u32) {
        match self {
            Packet::Send {header, ..} => header.checksum += value,
            Packet::Broadcast {header, ..} => header.checksum += value,
            Packet::Ack {header, ..} => header.checksum += value,
        }
    }

    pub fn get_dst_addr(&self) -> SocketAddr {
        match &self {
            Packet::Send {header, ..} => header.dst_addr,
            Packet::Broadcast {header, ..} => header.dst_addr,
            Packet::Ack {header, ..} => header.dst_addr,
        }
    }

    pub fn get_src_addr(&self) -> SocketAddr {
        match &self {
            Packet::Send {header, ..} => header.src_addr,
            Packet::Broadcast {header, ..} => header.src_addr,
            Packet::Ack {header, ..} => header.src_addr,
        }
    }

    pub fn get_origin_addr(&self) -> SocketAddr {
        match &self {
            Packet::Broadcast {header, ..} => header.origin,
            Packet::Send {header, ..} => header.src_addr,
            Packet::Ack {..} => { panic!("Ack packet must not have origin") },
        }
    }

    pub fn get_seq_num(&self) -> u32 {
        match &self {
            Packet::Send {header, ..} => header.seq_num,
            Packet::Broadcast {header, ..} => header.seq_num,
            Packet::Ack {header, ..} => header.seq_num,
        }
    }

    pub fn is_last(&self) -> bool {
        match self {
            Packet::Send {header, ..} => header.is_last,
            Packet::Broadcast {header, ..} => header.is_last,
            Packet::Ack {..} => { panic!("Ack packet must not have is_last") },
        }
    }

    pub fn get_data(&self) -> Vec<u8> {
        match self {
            Packet::Send {data, ..} => data.clone(),
            Packet::Broadcast {data, ..} => data.clone(),
            Packet::Ack {..} => { panic!("Ack packet must not have data") },
        }
    }
}



impl Packet {
    fn new_send(src_addr: SocketAddr, dst_addr: SocketAddr,
        seq_num: u32, is_last: bool, data: Vec<u8>) -> Self {
        let header = HeaderSend {
            src_addr,
            dst_addr,
            seq_num,
            is_last,
            checksum: 0,
        };
        Packet::new(Header::Send(header), data)
    }

    fn new_brdcst(src_addr: SocketAddr, dst_addr: SocketAddr,
        origin: SocketAddr, seq_num: u32, is_last: bool,
        data: Vec<u8>) -> Self {
        let header = HeaderBroadcast {
            src_addr,
            dst_addr,
            origin,
            seq_num,
            is_last,
            checksum: 0,
        };
        Packet::new(Header::Broadcast(header), data)
    }

    pub fn get_ack(&self) -> Self {
        let header = match self {
            Packet::Send {header, ..} => {
                header.get_ack()
            },
            Packet::Broadcast {header, ..} => {
                header.get_ack()
            },
            Packet::Ack {..} => { panic!("Ack packet must not have ack") },
        };
        Packet::Ack { header }
    }

    pub fn send_pkts_from_message(
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        seq_num: u32,
        data: Vec<u8>,
    ) -> Vec<Self> {
        let chunks: Vec<&[u8]> = data.chunks(Packet::BUFFER_SIZE - HeaderBroadcast::size()).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            Packet::new_send(
                src_addr,
                dst_addr,
                seq_num + i as u32,
                i == (chunks.len() - 1),
                chunk.to_vec(),
            )
        }).collect()
    }

    pub fn brdcst_pkts_from_message(
        src_addr: SocketAddr,
        dst_addr: SocketAddr,
        origin: SocketAddr,
        seq_num: u32,
        data: Vec<u8>,
    ) -> Vec<Self> {
        let chunks: Vec<&[u8]> = data.chunks(Packet::BUFFER_SIZE - HeaderBroadcast::size()).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            Packet::new_brdcst(
                src_addr,
                dst_addr,
                origin,
                seq_num + i as u32,
                i == (chunks.len() - 1),
                chunk.to_vec(),
            )
        }).collect()
    }
}
