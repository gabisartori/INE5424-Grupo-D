/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::UdpSocket;
use std::sync::Arc;

use crate::config::{BUFFER_SIZE, LOSS_RATE, CORRUPTION_RATE};
use super::packet::Packet;
// use super::failure_detection;

// Estrutura básica para a camada de comunicação por canais
#[derive(Clone)]
pub struct Channel {
    socket: Arc<UdpSocket>,
}

impl Channel {
    /// Constructor
    pub fn new(bind_addr: std::net::SocketAddr) -> Arc<Self> {
        let socket = Arc::new(UdpSocket::bind(bind_addr).unwrap());
        Arc::new(Self { socket })
    }

    /// Validates the received message
    /// For now, only validates the checksum
    fn validate_message(&self, packet: &Packet) -> bool {
        // Checksum
        let c1: bool = packet.header.checksum == Packet::checksum(&packet.header, &packet.data);
        c1
    }

    /// Reads a packet from the socket or waits for a packet to arrive
    pub fn receive(&self) -> Packet {
        loop {
            let mut buffer = [0; BUFFER_SIZE];
            let size;
            match self.socket.recv_from(&mut buffer) {
                Ok((size_, _)) => { size = size_; },
                Err(e) => {
                    let agent = self.socket.local_addr().unwrap().port() % 100;
                    debug_println!("->-> Erro {{{e}}} no Agente {agent} ao receber pacote pelo socket");
                    continue;
                },
            }
            let mut packet = Packet::from_bytes(buffer, size);
            // Simula perda de pacotes
            if rand::random::<f32>() < LOSS_RATE { continue; }
            if rand::random::<f32>() < CORRUPTION_RATE {
                packet.header.checksum += 1;
            }
            // Verifica se o pacote foi corrompido
            if !self.validate_message(&packet) { continue; }
            return packet
        }
    }

    /// Wrapper for UdpSocket::send_to, with some print statements
    pub fn send(&self, packet: &Packet) -> bool { 
        let agent_s = packet.header.src_addr.port() % 100;
        let agent_d = packet.header.dst_addr.port() % 100;
        let pk = packet.header.seq_num;
        let is_ack = packet.header.is_ack();
        match self.socket.send_to(&packet.to_bytes(), packet.header.dst_addr) {
            Ok(_) => true,
            Err(e) => {
                if is_ack {
                    debug_println!("->-> Erro {{{e}}} ao enviar ACK {pk} do Agente {agent_s} para o Agente {agent_d} pelo socket");
                }
                else {
                    debug_println!("->-> Erro {{{e}}} ao enviar pacote {pk} do Agente {agent_s} para o Agente {agent_d}");}
                false
            }
        }
    }
}
