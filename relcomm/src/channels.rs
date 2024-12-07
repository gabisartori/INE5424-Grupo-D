/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::UdpSocket;
use std::sync::Arc;

use crate::node::Node;
use crate::packet::Packet;
use crate::config::LOSS_RATE;
use crate::rec_aux::RecAux;
use logger::debug;
use logger::log::SharedLogger;

// Estrutura básica para a camada de comunicação por canais
#[derive(Clone)]
pub struct Channel {
    socket: Arc<UdpSocket>,
    logger: SharedLogger,
    host: Node
}

impl Channel {
    /// Constructor
    pub fn new(bind_addr: std::net::SocketAddr, logger: SharedLogger, host: Node) -> Result<Arc<Self>, std::io::Error> {
        let socket = Arc::new(UdpSocket::bind(bind_addr)?);
        Ok(Arc::new(Self { socket, logger, host }))
    }

    /// Validates the received message
    /// For now, only validates the checksum
    fn validate_message(&self, packet: &Packet) -> bool {
        // Checksum
        let c1: bool = packet.header.checksum == Packet::checksum(&packet.header, &packet.data);
        c1
    }

    /// Reads a packet from the socket or waits for a packet to arrive
    pub fn receive(&self) -> Result<Packet, std::io::Error> {
        loop {
            let mut buffer = [0; Packet::BUFFER_SIZE];
            let (size, _) = self.socket.recv_from(&mut buffer)?;

            let packet = match Packet::from_bytes(buffer, size) {
                Ok(packet) => packet,
                Err(e) => {
                    // I'm pretty sure this error will never happen, but I can't make it so
                    // that Packet::from_bytes builds the header from the buffer slice without checking if the size is correct
                    // Which it'll always be since the HEADER_SIZE is a constant
                    debug!("->-> Erro {{{e}}} ao receber pacote pelo socket");
                    continue;
                }
            };
            // Simula perda de pacotes, usand o parâmetro LOSS_RATE
            if rand::random::<f32>() < LOSS_RATE {
                Self::log_pkt(&self.logger, &self.host, &packet, logger::log::PacketStatus::InjectedFailure);
                debug!("Falha ao ler {}", packet);
                continue;
            }
            // Verifica se o pacote foi corrompido
            if !self.validate_message(&packet) { 
                Self::log_pkt(&self.logger, &self.host, &packet, logger::log::PacketStatus::ReceivedFailed);
                continue; 
            }

            return Ok(packet);
        }
    }

    /// Wrapper for UdpSocket::send_to, with some print statements
    pub fn send(&self, packet: &Packet) -> bool {
        // Simula perda de pacotes, usand o parâmetro LOSS_RATE
        if rand::random::<f32>() < LOSS_RATE {
            debug!("Falha ao enviar {}", packet);
            return false;
        }

        let agent_s = packet.header.src_addr.port() % 100;
        let agent_d = packet.header.dst_addr.port() % 100;
        let pk = packet.header.seq_num;
        let is_ack = packet.header.is_ack();

        match self.socket.send_to(&packet.to_bytes(), packet.header.dst_addr) {
            Ok(_) => true,
            Err(e) => {
                if is_ack {
                    debug!("->-> Erro {{{e}}} ao enviar ACK {pk} do Agente {agent_s} para o Agente {agent_d} pelo socket");
                    Self::log_pkt(&self.logger, &self.host, &packet, logger::log::PacketStatus::SentAckFailed);
                }
                else {
                    debug!("->-> Erro {{{e}}} ao enviar pacote {pk} do Agente {agent_s} para o Agente {agent_d}");}
                    Self::log_pkt(&self.logger, &self.host, &packet, logger::log::PacketStatus::SentFailed);
                false
            }
        }
    }
}
impl RecAux for Channel {}
