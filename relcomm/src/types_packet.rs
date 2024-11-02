// Importações necessárias
use std::net::SocketAddr;
use crate::types_header::{Header, Ack, HeaderBroadcast, HeaderSend, IsHeader};

/// Estrutura básica para pacotes
#[derive(Clone)]
pub enum PacketType {
    Send(SendPkt),
    Broadcast(BrdPkt),
    Ack(Ack),
}

#[derive(Clone)]
pub struct SendPkt {
    pub header: HeaderSend,
    pub data: Vec<u8>,
}
#[derive(Clone)]
pub struct BrdPkt {
    pub header: HeaderBroadcast,
    pub data: Vec<u8>,
}

/// essencial para Distribuição de pacotes
pub trait Packet {
    fn checksum(pkt: &Self) -> u32;
    fn from_bytes(bytes: Vec<u8>) -> Self;
    fn to_bytes(&self) -> Vec<u8>;
}

impl Packet for SendPkt {
    fn checksum(pkt: &Self) -> u32 {
        let sum: u32 = pkt.data.iter().fold(0, |acc, &byte| acc.wrapping_add(byte as u32));
        sum.wrapping_add(HeaderSend::checksum(&pkt.header))
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let data = bytes[HeaderSend::size()..].to_vec();
        let header = HeaderSend::from_bytes(bytes);
        SendPkt{header, data}
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.data);
        bytes
    }
}

impl Packet for BrdPkt {
    fn checksum(pkt: &Self) -> u32 {
        let sum: u32 = pkt.data.iter().fold(0, |acc, &byte| acc.wrapping_add(byte as u32));
        sum.wrapping_add(HeaderBroadcast::checksum(&pkt.header))
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let data = bytes[HeaderBroadcast::size()..].to_vec();
        let header = HeaderBroadcast::from_bytes(bytes);
        BrdPkt{header, data}
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.data);
        bytes
    }
}

impl Packet for Ack {
    fn checksum(pkt: &Self) -> u32 {
        <Ack as IsHeader>::checksum(pkt)
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        <Ack as IsHeader>::from_bytes(bytes)
    }

    fn to_bytes(&self) -> Vec<u8> {
        <Ack as IsHeader>::to_bytes(&self)
    }
}

pub trait FromBytes {
    fn from_bytes(bytes: Vec<u8>) -> Self;
}

impl FromBytes for PacketType {
    fn from_bytes(bytes: Vec<u8>) -> Self {
        let type_h = bytes[0];
        let bytes = bytes[1..].to_vec();
        match type_h {
            Header::SD => {
                PacketType::Send(SendPkt::from_bytes(bytes))
            },
            Header::BD => {
                PacketType::Broadcast(BrdPkt::from_bytes(bytes))
            },
            Header::ACK => {
                PacketType::Ack(<Ack as IsHeader>::from_bytes(bytes))
            },
            _ => panic!("Invalid packet type"),
        }
    }

}

pub trait Get: FromBytes {
    const BUFFER_SIZE: usize = 2<<9;
    fn get_checksum(&self) -> u32;
    fn get_dst_addr(&self) -> SocketAddr;
    fn get_src_addr(&self) -> SocketAddr;
    fn get_origin_addr(&self) -> SocketAddr;
    fn get_seq_num(&self) -> u32;
    fn is_last(&self) -> bool;
}
/// getters
impl Get for PacketType {
    fn get_checksum(&self) -> u32 {
        match self {
            PacketType::Send(pkt) => pkt.header.checksum,
            PacketType::Broadcast(pkt) => pkt.header.checksum,
            PacketType::Ack(pkt) => pkt.checksum,
        }
    }

    fn get_dst_addr(&self) -> SocketAddr {
        match &self {
            PacketType::Send (pkt) => pkt.header.dst_addr,
            PacketType::Broadcast (pkt) => pkt.header.dst_addr,
            PacketType::Ack (pkt) => pkt.dst_addr,
        }
    }

    fn get_src_addr(&self) -> SocketAddr {
        match &self {
            PacketType::Send (pkt) => pkt.header.src_addr,
            PacketType::Broadcast (pkt) => pkt.header.src_addr,
            PacketType::Ack (pkt) => pkt.src_addr,
        }
    }

    fn get_origin_addr(&self) -> SocketAddr {
        match &self {
            PacketType::Broadcast (pkt) => pkt.header.origin,
            PacketType::Send (pkt) => pkt.header.src_addr,
            PacketType::Ack {..} => { panic!("Ack packet must not have origin") },
        }
    }

    fn get_seq_num(&self) -> u32 {
        match &self {
            PacketType::Send (pkt) => pkt.header.seq_num,
            PacketType::Broadcast (pkt) => pkt.header.seq_num,
            PacketType::Ack (pkt) => pkt.seq_num,
        }
    }

    fn is_last(&self) -> bool {
        match self {
            PacketType::Send (pkt) => pkt.header.is_last,
            PacketType::Broadcast (pkt) => pkt.header.is_last,
            PacketType::Ack {..} => { panic!("Ack packet must not have is_last") },
        }
    }
}

/// checksum and to_bytes interface, and get_data
impl PacketType {
    pub fn checksum(pkt: &Self) -> u32 {
        match pkt {
            PacketType::Send(pkt) => SendPkt::checksum(pkt),
            PacketType::Broadcast(pkt) => BrdPkt::checksum(pkt),
            PacketType::Ack(pkt) => <Ack as IsHeader>::checksum(pkt),
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PacketType::Send(pkt) => pkt.to_bytes(),
            PacketType::Broadcast(pkt) => pkt.to_bytes(),
            PacketType::Ack(pkt) => <Ack as IsHeader>::to_bytes(pkt),
        }
    }

    pub fn get_data(&self) -> &Vec<u8> {
        match self {
            PacketType::Send(pkt) => pkt.get_data(),
            PacketType::Broadcast(pkt) => pkt.get_data(),
            PacketType::Ack(_) => { panic!("Ack packet must not have data") },
        }
    }
}

/// setter do checksum
pub trait Set {
    fn set_checksum(&mut self, value: u32);
}
impl Set for SendPkt {
    fn set_checksum(&mut self, value: u32) {
        self.header.checksum += value;
    }
}
impl Set for BrdPkt {
    fn set_checksum(&mut self, value: u32) {
        self.header.checksum += value;
    }
}
impl Set for Ack {
    fn set_checksum(&mut self, value: u32) {
        self.checksum += value;
    }
}
impl Set for PacketType {
    fn set_checksum(&mut self, value: u32) {
        match self {
            PacketType::Send(pkt) => pkt.set_checksum(value),
            PacketType::Broadcast(pkt) => pkt.set_checksum(value),
            PacketType::Ack(pkt) => pkt.set_checksum(value),
        }
    }
}
/// usados pelos pacotes que contêm dados
pub trait HasData: Sized + Packet {
    fn new(header: Header, data: Vec<u8>) -> Self;    
    fn get_data(&self) -> &Vec<u8>;
    fn get_ack(&self) -> PacketType;
    fn pkts_from_msg(addrs: Vec<SocketAddr>, seq_num: u32, data: Vec<u8>) -> Vec<PacketType>;
}

impl HasData for SendPkt {
    fn new(header: Header, data: Vec<u8>) -> Self {
        let mut pkt = match header {
            Header::Send(header) => {
                SendPkt {
                    header,
                    data,
                }
            },
            _ => { panic!("Invalid header type") },
        };
        pkt.set_checksum(SendPkt::checksum(&pkt));
        pkt
    }
    fn get_ack(&self) -> PacketType {
        PacketType::Ack(self.header.get_ack())
    }

    fn pkts_from_msg(addrs: Vec<SocketAddr>, seq_num: u32, data: Vec<u8>) -> Vec<PacketType> {
        let chunks: Vec<&[u8]> = data.chunks(PacketType::BUFFER_SIZE - HeaderSend::size()).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {            
            PacketType::Send(SendPkt::new(
                Header::Send(HeaderSend {
                        src_addr: addrs[0],
                        dst_addr: addrs[1],
                        seq_num: seq_num + i as u32,
                        is_last: i == (chunks.len() - 1),
                        checksum: 0,
                        }
                    ),
                chunk.to_vec(),
            ))
        }).collect()
    }

    fn get_data(&self) -> &Vec<u8> {
        &self.data
    }
}

impl HasData for BrdPkt {
    fn new(header: Header, data: Vec<u8>) -> Self {
        let mut pkt = match header {
            Header::Broadcast(header) => {
                BrdPkt {
                    header,
                    data,
                }
            },
            _ => { panic!("Wrong Header type") },
        };
        pkt.set_checksum(Packet::checksum(&pkt));
        pkt
    }

    fn get_ack(&self) -> PacketType {
        PacketType::Ack(self.header.get_ack())
    }

    fn pkts_from_msg(addrs: Vec<SocketAddr>, seq_num: u32, data: Vec<u8>) -> Vec<PacketType> {
        let chunks: Vec<&[u8]> = data.chunks(PacketType::BUFFER_SIZE - HeaderSend::size()).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            let header = Header::Broadcast(
                HeaderBroadcast {
                        src_addr: addrs[0],
                        dst_addr: addrs[1],
                        origin: addrs[2],
                        seq_num: seq_num + i as u32,
                        is_last: i == (chunks.len() - 1),
                        checksum: 0
                        }
            );
            PacketType::Broadcast(BrdPkt::new(
                header,
                chunk.to_vec(),
            ))
        }).collect()
    }

    fn get_data(&self) -> &Vec<u8> {
        &self.data
    }
}
