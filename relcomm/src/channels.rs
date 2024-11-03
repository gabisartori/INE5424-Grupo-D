/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::UdpSocket;
use std::sync::Arc;
use lazy_static::lazy_static;
use std::io::Error;
// use crate::packet::Packet;
use crate::types_packet::{PacketType, Packet, Get, FromBytes, Set};

/// reads the loss_rate and corruption_rate from the command line arguments
/// they should be the last two arguments exept for the agent and test id
fn get_args() -> (f32, f32) {
    let args: Vec<String> = std::env::args().collect();
    let size = args.len() - 3;
    let loss_rate = args[size].parse::<f32>().unwrap();
    let corruption_rate = args[size - 1].parse::<f32>().unwrap();
    (loss_rate, corruption_rate)
}

// global variables to read the loss_rate and corruption_rate
lazy_static! {
    static ref LOSS_RATE: f32 = get_args().0;
    static ref CORRUPTION_RATE: f32 = get_args().1;
}

// Estrutura básica para a camada de comunicação por canais
#[derive(Clone)]
pub struct Channel {
    socket: Arc<UdpSocket>,
}

impl Channel {
    /// Constructor
    pub fn new(bind_addr: std::net::SocketAddr) -> Result<Arc<Self>, std::io::Error> {
        let socket = Arc::new(UdpSocket::bind(bind_addr)?);
        Ok(Arc::new(Self { socket }))
    }

    /// Validates the received message
    /// For now, only validates the checksum
    fn validate_message(&self, packet: &PacketType) -> bool {
        // Checksum
        let c1: bool = packet.get_checksum() == PacketType::checksum(packet);
        c1
    }

    /// Reads a packet from the socket or waits for a packet to arrive
    pub fn receive(&self) -> Result<PacketType, Error> {
        loop {
            let mut buffer = [0; PacketType::BUFFER_SIZE];
            let (size, _) = self.socket.recv_from(&mut buffer)?;
            let buffer = buffer[..size].to_vec();

            let mut packet = PacketType::from_bytes(buffer);
            // Simula perda de pacotes, usand as referências staticas LOSS_RATE e CORRUPTION_RATE
            if rand::random::<f32>() < *LOSS_RATE {
                continue;
            }
            if rand::random::<f32>() < *CORRUPTION_RATE {
                packet.set_checksum(1);
            }
            // Verifica se o pacote foi corrompido
            if !self.validate_message(&packet) { continue; }
            return Ok(packet);
        }
    }

    /// Wrapper for UdpSocket::send_to, with some print statements
    pub fn send<P>(&self, packet: &P) -> bool
        where P: Get + Packet {
        match self.socket.send_to(&packet.to_bytes(), packet.get_dst_addr()) {
            Ok(_) => true,
            Err(_) => false
        }
    }
}
