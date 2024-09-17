/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/

use crate::config::BUFFER_SIZE;
// use super::failure_detection;

use std::f32::consts::E;
// Importações necessárias
use std::net::{UdpSocket, SocketAddr, IpAddr, Ipv4Addr, Ipv6Addr};
use std::io::Error;
use std::process::Output;

// sempre deve-se alterar o tamanho do cabeçalho se alterar o Header
const HEADER_SIZE: usize = 30; // Header::new_empty().to_bytes().len()
// estrutura para o cabeçalho
pub struct Header {
    pub src_addr: SocketAddr,
    pub dst_addr: SocketAddr,
    pub ack_num: u32,
    pub seq_num: u32,
    pub msg_size: usize,
    pub flags: u8,
    pub is_last: bool,
    // um vetor mutável de bytes
    pub msg: Vec<u8>,
}
// implementação para que o cabeçalho seja conversível em bytes e vice-versa
impl<'a> Header {
    pub fn new_empty() -> Self {
        Self {
            src_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            dst_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
            ack_num: 0,
            seq_num: 0,
            msg_size: 0,
            flags: 0,
            is_last: false,
            // a mensagem é uma array de bytes vazio
            msg: Vec::new(),
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
        bytes.push(self.flags);
        bytes.push(self.is_last as u8);
        bytes.extend_from_slice(self.msg.as_slice());
        bytes
    }

    pub fn from_bytes(&mut self, mut bytes: [u8; BUFFER_SIZE+HEADER_SIZE]){
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
        let flags = bytes[28];
        let is_last = bytes[29] == 1;
        let mut msg = bytes[HEADER_SIZE..].to_vec();
        
        self.src_addr = src_addr;
        self.dst_addr = dst_addr;
        self.ack_num = ack_num;
        self.seq_num = seq_num;
        self.msg_size = msg_size;
        self.flags = flags;
        self.is_last = is_last;
        self.msg = msg;
    }
}

// Estrutura básica para a camada de comunicação por canais
pub struct Channel {
    socket: UdpSocket,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(bind_addr: &SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        // socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }

    // Função para enviar mensagem para um destinatário específico
    pub fn send(&self, header: Header) -> Result<(), Error> {

        self.socket.send_to(&header.to_bytes(), header.dst_addr)?;
        Ok(())
    }

    // Função para receber mensagens
    pub fn receive(&self, header: &mut Header) -> Result<(usize, SocketAddr), Error> {
        let mut buffer = [0; BUFFER_SIZE+HEADER_SIZE];
        let info = self.socket.recv_from(&mut buffer);
        match info {
            Ok((size, src)) => {
                header.from_bytes(buffer);
                Ok((size, src))
            }
            Err(_) => {
                Ok((0, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
            }
            
        }        
    }
}
