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
use std::sync::mpsc;
use std::thread;

// sempre deve-se alterar o tamanho do cabeçalho se alterar o Header
const HEADER_SIZE: usize = 32; // Header::new_empty().to_bytes().len()
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
}

// Estrutura básica para a camada de comunicação por canais
pub struct Channel {
    socket: UdpSocket,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(bind_addr: &SocketAddr, input_rx: mpsc::Receiver<Header>) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        // Instantiate sender and listener threads
        // And create a channel to communicate between them
        // MPSC: Multi-Producer Single-Consumer
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || Channel::sender(input_rx, rx));
        thread::spawn(move || Channel::listener(tx));
        Ok(Self { socket})
    }

    fn sender(rel_comm_rx: mpsc::Receiver<Header>, listener_rx: mpsc::Receiver<Header>) {
        loop {
            for header in rel_comm_rx.try_iter() {
                // Add message to list of waiting acks

                // Send message through the socket
                
            }
            for header in listener_rx.try_iter() {
                // Wait for acks
                continue;
            } 
        }
    }

    fn listener(tx: mpsc::Sender<Header>) {
        loop {
            let message = [0; BUFFER_SIZE+HEADER_SIZE];
            match self.socket.recv_from(&mut message) {
                Ok((size, src)) => {
                    let mut header = Header::new_empty();
                    header.from_bytes(message);
                    tx.send(header).unwrap();
                }
                Err(_) => {
                    continue;
                }
            }
            // Process message
            // If it's an ACK, send it to the sender
            // If it's a message, keep it to itself and send an ACK
        }
    }

    pub fn receive(&self) {
        return;
        // If there's a message ready, return it
        // Else: block and wait for there to be a
    } 
}
