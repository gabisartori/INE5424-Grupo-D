/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/

use crate::config::BUFFER_SIZE;
// use super::failure_detection;

// Importações necessárias
use std::net::{UdpSocket, SocketAddr};
use std::io::Error;

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
    pub fn send(&self, dst_addr: &SocketAddr, message: &[u8]) -> Result<(), Error> {
        self.socket.send_to(message, dst_addr)?;
        Ok(())
    }

    // Função para receber mensagens
    pub fn receive(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        let (size, src) = self.socket.recv_from(buffer)?;
        Ok((size, src))
    }
}
