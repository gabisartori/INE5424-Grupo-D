/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::UdpSocket;
use std::sync::Arc;
use std::io::Error;
use crate::config::LOSS_RATE;
use crate::types_packet::{PacketType, Packet, Get};

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

    /// Reads a packet from the socket or waits for a packet to arrive
    pub fn receive(&self) -> Result<PacketType, Error> {
        loop {
            let mut buffer = [0; PacketType::BUFFER_SIZE];
            let (size, _) = self.socket.recv_from(&mut buffer)?;
            let buffer = buffer[..size].to_vec();

            let packet = PacketType::from_bytes(buffer);
            // Simula perda de pacotes, usand o parâmetro LOSS_RATE
            if rand::random::<f32>() < LOSS_RATE {
                continue;
            }
            // Verifica se o pacote foi corrompido
            if !packet.validate_pkt() { continue; }
            return Ok(packet);
        }
    }

    /// Wrapper for UdpSocket::send_to, but with packets
    pub fn send(&self, packet: &(impl Packet + Get)) -> bool {
        if rand::random::<f32>() < LOSS_RATE {
            return false;
        }
        match self.socket.send_to(&packet.to_bytes(), packet.get_dst_addr()) {
            Ok(_) => true,
            Err(_) => false
        }
    }
}
