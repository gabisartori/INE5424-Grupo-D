/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::{Channel, Header};
use crate::config::{BUFFER_SIZE, Node, TIMEOUT, W_SIZE};

use std::net::SocketAddr;

pub struct ReliableCommunication {
    pub channel: Channel,
    pub host: SocketAddr,
    pub group: Vec<Node>,
}

// TODO: Fazer com que a inicialização seja de um grupo

impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: SocketAddr, group: Vec<Node>) -> Self {
        let channel = Channel::new(&host)
        .expect("\nFalha ao inicializar o canal no nível Rel_Com\n");
        Self { channel: channel, host: host, group: group }
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: &[u8]) {
        let mut base = 0;
        let mut buffer_acks: [u8; W_SIZE] = [0; W_SIZE];
        let packages: Vec<&[u8]> = message.chunks(BUFFER_SIZE).collect();
        let mut next_seq_num = 0;
        loop {
            if next_seq_num < base + W_SIZE && next_seq_num < packages.len() {
                // let mut pck: [u8] = packages[next_seq_num];
                let mut msg = packages[next_seq_num].to_vec();
                let mut msg = msg.as_mut_slice();
                // enviando pacotes da janela
                self.raw_send(&dst_addr, &mut msg);
                next_seq_num += 1;
            } else {
                // Lógica para confirmar recebimento de ACKs
                let mut timer = std::time::Instant::now();
                loop {
                    if timer.elapsed().as_millis() < TIMEOUT {
                        let mut ack_buffer = [0; BUFFER_SIZE];
                        let (size, src) = self.receive(&mut ack_buffer);
                        let ack = ack_buffer[0] as usize;
                        if ack == base {
                            base += 1;
                        } else {
                            next_seq_num = base;                        
                            break;
                        }
                        if base == next_seq_num {
                            break;
                        }
                    } else { // timeout estourado
                        next_seq_num = base;
                        break;
                    }
                }

                if next_seq_num >= packages.len() {
                    break;
                }
            }
        }
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut [u8]) -> (usize, SocketAddr) {
        let mut header = Header::new_empty();
        let (size, src) =  self.channel.receive(&mut header)
        .expect("Falha ao receber mensagem no nível Rel_Com\n");
        buffer.copy_from_slice(&header.msg);
        // Lógica para validar e garantir a entrega confiável
        if self.validate_message(buffer) {
            (size, src)
        } else {
            (0, src)
        }
    }

    fn validate_message(&self, message: &[u8]) -> bool {
        // Lógica para validar a mensagem recebida
        true
    }

    fn raw_send(&self, dst_addr: &SocketAddr, message: &mut [u8]) {
        let header = Header {
            src_addr: self.host,
            dst_addr: *dst_addr,
            ack_num: 0,
            seq_num: 0,
            msg_size: message.len(),
            flags: 0,
            msg: message.to_vec(),
        };
        self.channel.send(header)
        .expect("Falha ao enviar mensagem no nível Rel_Com\n");
    }
}

