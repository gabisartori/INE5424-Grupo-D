/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

use std::net::SocketAddr;

// Importa a camada de canais
use super::channels::Channel;
use crate::config::BUFFER_SIZE;

pub struct ReliableCommunication {
    // a hashmap of channels, using the string address as the key
    channel: Channel
}

impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(addr: &SocketAddr) -> Self {
        let channel = Channel::initialize(&addr).expect("\nFalha ao inicializar o canal no nível Rel_Com\n");
        Self { channel: channel }
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: &[u8; BUFFER_SIZE]) {
        // Implementação da lógica de comunicação confiável (ex.: retransmissão, ACKs, etc.)
        self.channel.send(&dst_addr, message).expect("Falha ao enviar mensagem no nível Rel_Com\n");
        // Lógica para lidar com confirmação de entrega e retransmissão
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut [u8; BUFFER_SIZE]) -> (usize, SocketAddr) {
        self.channel.receive(buffer).expect("Falha ao receber mensagem no nível Rel_Com\n")

        // Lógica para validar e garantir a entrega confiável
        // buffer[..size].to_vec() // Retorna a mensagem recebida
    }
}

