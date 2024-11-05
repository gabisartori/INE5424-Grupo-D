// Importações necessárias
use std::net::SocketAddr;
use crate::types_header::{HBrd, HSnd, HLRq, Ack, AckBrd, Header, DataHeader, IsAck};
/// Estrutura básica para pacotes com uma mensagem
#[derive(Clone)]
pub struct DataPkt<H: DataHeader> {
    pub header: H,
    pub data: Vec<u8>,
}

/// Estrutura básica para pacotes
#[derive(Clone)]
pub enum PacketType {
    Send(DataPkt<HSnd>),
    Broadcast(DataPkt<HBrd>),
    LeaderRequest(DataPkt<HLRq>),
    Ack(Ack),
    AckBrd(AckBrd),
}

/// getters para os campos essenciais dos pacotes
pub trait Get {
    const BUFFER_SIZE: usize = 2<<9;
    fn get_checksum(&self) -> u32;
    fn get_dst_addr(&self) -> SocketAddr;
    fn get_src_addr(&self) -> SocketAddr;
    fn get_origin_addr(&self) -> SocketAddr;
    fn get_seq_num(&self) -> u32;
    fn type_h(&self) -> &str;
}

impl Get for PacketType {
    fn get_checksum(&self) -> u32 {
        match self {
            PacketType::Send(pkt) => pkt.header.checksum,
            PacketType::Broadcast(pkt) => pkt.header.checksum,
            PacketType::LeaderRequest(pkt) => pkt.header.checksum,
            PacketType::Ack(pkt) => pkt.checksum,
            PacketType::AckBrd(pkt) => pkt.checksum,
        }
    }

    fn get_dst_addr(&self) -> SocketAddr {
        match &self {
            PacketType::Send (pkt) => pkt.header.dst_addr,
            PacketType::Broadcast (pkt) => pkt.header.dst_addr,
            PacketType::LeaderRequest (pkt) => pkt.header.dst_addr,
            PacketType::Ack (pkt) => pkt.dst_addr,
            PacketType::AckBrd (pkt) => pkt.dst_addr,
        }
    }

    fn get_src_addr(&self) -> SocketAddr {
        match &self {
            PacketType::Send (pkt) => pkt.header.src_addr,
            PacketType::Broadcast (pkt) => pkt.header.src_addr,
            PacketType::LeaderRequest (pkt) => pkt.header.src_addr,
            PacketType::Ack (pkt) => pkt.src_addr,
            PacketType::AckBrd (pkt) => pkt.src_addr,
        }
    }

    fn get_origin_addr(&self) -> SocketAddr {
        match &self {
            PacketType::Broadcast (pkt) => pkt.header.origin,
            PacketType::Send (pkt) => pkt.header.src_addr,
            PacketType::LeaderRequest (pkt) => pkt.header.src_addr,
            PacketType::AckBrd (pkt) => pkt.origin,
            PacketType::Ack {..} => { panic!("Ack packet must not have origin") },
        }
    }

    fn get_seq_num(&self) -> u32 {
        match &self {
            PacketType::Send (pkt) => pkt.header.seq_num,
            PacketType::Broadcast (pkt) => pkt.header.seq_num,
            PacketType::LeaderRequest (pkt) => pkt.header.seq_num,
            PacketType::Ack (pkt) => pkt.seq_num,
            PacketType::AckBrd (pkt) => pkt.seq_num,
        }
    }

    fn type_h(&self) -> &str {
        match self {
            PacketType::Ack(pkt) => pkt.type_h(),
            PacketType::AckBrd(pkt) => pkt.type_h(),
            PacketType::Send(pkt) => pkt.type_h(),
            PacketType::Broadcast(pkt) => pkt.type_h(),
            PacketType::LeaderRequest(pkt) => pkt.type_h(),
        }
    }
}

impl Get for DataPkt<HSnd> {
    fn get_checksum(&self) -> u32 {
        self.header.checksum
    }

    fn get_dst_addr(&self) -> SocketAddr {
        self.header.dst_addr
    }

    fn get_src_addr(&self) -> SocketAddr {
        self.header.src_addr
    }

    fn get_origin_addr(&self) -> SocketAddr {
        self.header.src_addr
    }

    fn get_seq_num(&self) -> u32 {
        self.header.seq_num
    }

    fn type_h(&self) -> &str {
        HSnd::TYPE
    }
}

impl Get for DataPkt<HBrd> {
    fn get_checksum(&self) -> u32 {
        self.header.checksum
    }

    fn get_dst_addr(&self) -> SocketAddr {
        self.header.dst_addr
    }

    fn get_src_addr(&self) -> SocketAddr {
        self.header.src_addr
    }

    fn get_origin_addr(&self) -> SocketAddr {
        self.header.origin
    }

    fn get_seq_num(&self) -> u32 {
        self.header.seq_num
    }

    fn type_h(&self) -> &str {
        HBrd::TYPE
    }    
}

impl Get for DataPkt<HLRq> {
    fn get_checksum(&self) -> u32 {
        self.header.checksum
    }

    fn get_dst_addr(&self) -> SocketAddr {
        self.header.dst_addr
    }

    fn get_src_addr(&self) -> SocketAddr {
        self.header.src_addr
    }

    fn get_origin_addr(&self) -> SocketAddr {
        self.header.src_addr
    }

    fn get_seq_num(&self) -> u32 {
        self.header.seq_num
    }

    fn type_h(&self) -> &str {
        HLRq::TYPE
    }
}

impl Get for Ack {
    fn get_checksum(&self) -> u32 {
        self.checksum
    }

    fn get_dst_addr(&self) -> SocketAddr {
        self.dst_addr
    }

    fn get_src_addr(&self) -> SocketAddr {
        self.src_addr
    }

    fn get_origin_addr(&self) -> SocketAddr {
        self.src_addr
    }

    fn get_seq_num(&self) -> u32 {
        self.seq_num
    }

    fn type_h(&self) -> &str {
        Self::TYPE
    }    
}

impl Get for AckBrd {
    fn get_checksum(&self) -> u32 {
        self.checksum
    }

    fn get_dst_addr(&self) -> SocketAddr {
        self.dst_addr
    }

    fn get_src_addr(&self) -> SocketAddr {
        self.src_addr
    }

    fn get_origin_addr(&self) -> SocketAddr {
        self.origin
    }

    fn get_seq_num(&self) -> u32 {
        self.seq_num
    }

    fn type_h(&self) -> &str {
        Self::TYPE
    }    
}

/// Para a conversão de bytes para pacotes
pub trait FromBytes {
    fn from_bytes(bytes: Vec<u8>) -> Self;
}

impl FromBytes for PacketType {
    fn from_bytes(bytes: Vec<u8>) -> Self {
        let type_h = bytes[0];
        let bytes = bytes[1..].to_vec();
        match type_h {
            Ack::SD => {
                PacketType::Send(DataPkt::from_bytes(bytes))
            },
            Ack::BD => {
                PacketType::Broadcast(DataPkt::from_bytes(bytes))
            },
            Ack::ACK => {
                PacketType::Ack(<Ack as Header>::from_bytes(bytes))
            },
            Ack::ACK_BD => {
                PacketType::AckBrd(<AckBrd as Header>::from_bytes(bytes))
            },
            Ack::LREQ => {
                PacketType::LeaderRequest(DataPkt::from_bytes(bytes))
            },
            _ => panic!("Invalid packet type"),
        }
    }
}

/// checksum and to_bytes interface, and get_data
impl PacketType {
    pub fn checksum(pkt: &Self) -> u32 {
        match pkt {
            PacketType::Ack(pkt) => <Ack as Header>::checksum(pkt),
            PacketType::Send(pkt) => DataPkt::<HSnd>::checksum(pkt),
            PacketType::Broadcast(pkt) => DataPkt::<HBrd>::checksum(pkt),
            PacketType::AckBrd(pkt) => <AckBrd as Header>::checksum(pkt),
            PacketType::LeaderRequest(pkt) => DataPkt::<HLRq>::checksum(pkt),
        }
    }
}


/// essencial para Distribuição de pacotes
pub trait Packet {
    type Header: Header;
    fn checksum(pkt: &Self) -> u32;
    fn from_bytes(bytes: Vec<u8>) -> Self;
    fn to_bytes(&self) -> Vec<u8>;
}

impl<A> Packet for A where A: IsAck + Header {
    type Header = A;
    fn checksum(pkt: &Self) -> u32 {
        A::checksum(pkt)
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        A::from_bytes(bytes)
    }

    fn to_bytes(&self) -> Vec<u8> {
        A::to_bytes(&self)
    }
}

impl<H> Packet for DataPkt<H> where H: DataHeader + Header {
    type Header = H;
    fn checksum(pkt: &Self) -> u32 {
        let sum: u32 = pkt.data.iter().fold(0, |acc, &byte| acc.wrapping_add(byte as u32));
        sum.wrapping_add(H::checksum(&pkt.header))
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let data = bytes[H::size()..].to_vec();
        let header = H::from_bytes(bytes);
        DataPkt{header, data}
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.data);
        bytes
    }
}

// Para evitar boilerplate, implementa HasData para pacotes que contêm dados
pub trait HasData<H: DataHeader>: Sized + Packet {
    type AckType: IsAck + Get;
    fn new(header: H, data: Vec<u8>) -> Self;
    fn pkts_from_msg(addrs: Vec<&SocketAddr>, seq_num: u32, data: Vec<u8>) -> Vec<Self>;
    fn plain_new(header: H, data: Vec<u8>) -> Self;
    fn get_ack(&self) -> Self::AckType;
}

impl<H> HasData<H> for DataPkt<H>
where
    H: DataHeader,
    DataPkt<H>: Set + Get,
    H::AckType: IsAck + Get,{
    type AckType = H::AckType;

    fn new(header: H, data: Vec<u8>) -> Self {
        let mut pkt = DataPkt::<H>::plain_new(header, data);
        pkt.set_checksum(Packet::checksum(&pkt));
        pkt
    }

    fn pkts_from_msg(addrs: Vec<&SocketAddr>, seq_num: u32, data: Vec<u8>) -> Vec<Self> {
        let chunks: Vec<&[u8]> = data.chunks(Self::BUFFER_SIZE - H::size()).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            let header = H::new(&addrs, seq_num + i as u32, i == (chunks.len() - 1));
            DataPkt::<H>::new(
                header,
                chunk.to_vec(),
            )
        }).collect()        
    }

    fn plain_new(header: H, data: Vec<u8>) -> Self {
        DataPkt {
            header,
            data,
        }
    }

    fn get_ack(&self) -> Self::AckType {
        self.header.get_ack()
    }
}

/// setter do checksum
pub trait Set {
    fn set_checksum(&mut self, value: u32);
}
impl Set for DataPkt<HSnd> {
    fn set_checksum(&mut self, value: u32) {
        self.header.checksum += value;
    }
}
impl Set for DataPkt<HBrd> {
    fn set_checksum(&mut self, value: u32) {
        self.header.checksum += value;
    }
}
impl Set for DataPkt<HLRq> {
    fn set_checksum(&mut self, value: u32) {
        self.header.checksum += value;
    }
}
impl Set for Ack {
    fn set_checksum(&mut self, value: u32) {
        self.checksum += value;
    }
}

impl Set for AckBrd {
    fn set_checksum(&mut self, value: u32) {
        self.checksum += value;
    }
}

impl Set for PacketType {
    fn set_checksum(&mut self, value: u32) {
        match self {
            PacketType::Ack(pkt) => pkt.set_checksum(value),
            PacketType::AckBrd(pkt) => pkt.set_checksum(value),
            PacketType::Send(pkt) => pkt.set_checksum(value),
            PacketType::Broadcast(pkt) => pkt.set_checksum(value),
            PacketType::LeaderRequest(pkt) => pkt.set_checksum(value),
        }
    }
}
