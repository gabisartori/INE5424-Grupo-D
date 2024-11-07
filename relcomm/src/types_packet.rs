use std::net::SocketAddr;
use crate::types_header::{HBrd, HSnd, HLRq, Ack, AckBrd, Header, DataHeader, IsAck};

/// Estrutura básica para pacotes com uma mensagem
#[derive(Clone)]
pub struct DataPkt<H: DataHeader> {
    pub header: H,
    pub data: Vec<u8>,
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
    fn get_ack(&self) -> Self::AckType;
}

impl<H> HasData<H> for DataPkt<H>
where
    H: DataHeader,
    DataPkt<H>: Get,
    H::AckType: IsAck + Get,{
    type AckType = H::AckType;

    fn new(header: H, data: Vec<u8>) -> Self {
        let mut pkt = DataPkt {
            header,
            data,
        };
        pkt.header.set_checksum(Packet::checksum(&pkt));
        pkt
    }

    fn pkts_from_msg(addrs: Vec<&SocketAddr>, seq_num: u32, data: Vec<u8>) -> Vec<Self> {
        let chunks: Vec<&[u8]> = data.chunks(PacketType::BUFFER_SIZE - H::size()).collect();

        chunks.iter().enumerate().map(|(i, chunk)| {
            let header = H::new(&addrs, seq_num + i as u32, i == (chunks.len() - 1));
            DataPkt::<H>::new(
                header,
                chunk.to_vec(),
            )
        }).collect()        
    }

    fn get_ack(&self) -> Self::AckType {
        self.header.get_ack()
    }
}

/// Estrutura básica para listar os tipos de pacotes e permitir a leitura from bytes
pub enum PacketType {
    Send(DataPkt<HSnd>),
    Broadcast(DataPkt<HBrd>),
    LeaderRequest(DataPkt<HLRq>),
    Ack(Ack),
    AckBrd(AckBrd),
}

/// Allows for conversion from bytes to packets and checksum validation
/// Also contains a buffer size for packetsw
impl PacketType {
    pub const BUFFER_SIZE: usize = 2<<9;
    /// Para a conversão de bytes para pacotes
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
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
            Ack::LREQ => {
                PacketType::LeaderRequest(DataPkt::from_bytes(bytes))
            },
            Ack::ACK_BD => {
                PacketType::AckBrd(<AckBrd as Header>::from_bytes(bytes))
            },
            _ => panic!("Invalid packet type"),
        }
    }

    /// Confere o checksum do pacote com o cálculo certo
    pub fn validate_pkt(&self) -> bool {
        match self {
            PacketType::Ack(pkt) => pkt.checksum == Packet::checksum(pkt),
            PacketType::AckBrd(pkt) => pkt.checksum == Packet::checksum(pkt),
            PacketType::Send(pkt) => pkt.header.checksum == Packet::checksum(pkt),
            PacketType::Broadcast(pkt) => pkt.header.checksum == Packet::checksum(pkt),
            PacketType::LeaderRequest(pkt) => pkt.header.checksum == Packet::checksum(pkt),
        }
    }
}

/// getters para os campos essenciais dos pacotes
pub trait Get: Packet {
    fn get_dst_addr(&self) -> SocketAddr;
    fn get_src_addr(&self) -> SocketAddr;
    fn get_seq_num(&self) -> u32;
    fn type_h(&self) -> &str;
}

/// a macro that implements the trait Get for a Packet
macro_rules! impl_get {
    ($t:ty) => {
        impl Get for $t {
            fn get_dst_addr(&self) -> SocketAddr {
                self.dst_addr
            }

            fn get_src_addr(&self) -> SocketAddr {
                self.src_addr
            }

            fn get_seq_num(&self) -> u32 {
                self.seq_num
            }

            fn type_h(&self) -> &str {
                Self::TYPE
            }
        }
    };
    ($t:ty, $h:ty) => {
        impl Get for $t {
            fn get_dst_addr(&self) -> SocketAddr {
                self.header.dst_addr
            }

            fn get_src_addr(&self) -> SocketAddr {
                self.header.src_addr
            }

            fn get_seq_num(&self) -> u32 {
                self.header.seq_num
            }

            fn type_h(&self) -> &str {
                <$h>::TYPE
            }
        }
    };
}

impl_get!(Ack);
impl_get!(AckBrd);
impl_get!(DataPkt<HSnd>, HSnd);
impl_get!(DataPkt<HBrd>, HBrd);
impl_get!(DataPkt<HLRq>, HLRq);

/// getter do campo origin, que pode não ser o mesmo que src_addr
pub trait Origin {
    fn get_origin_addr(&self) -> SocketAddr;
}

impl Origin for DataPkt<HSnd> {
    fn get_origin_addr(&self) -> SocketAddr {
        self.header.src_addr
    }
}

impl Origin for DataPkt<HLRq> {
    fn get_origin_addr(&self) -> SocketAddr {
        self.header.src_addr
    }
}

impl Origin for Ack {
    fn get_origin_addr(&self) -> SocketAddr {
        self.src_addr
    }
}

impl Origin for DataPkt<HBrd> {
    fn get_origin_addr(&self) -> SocketAddr {
        self.header.origin
    }
}

impl Origin for AckBrd {
    fn get_origin_addr(&self) -> SocketAddr {
        self.origin
    }
}
