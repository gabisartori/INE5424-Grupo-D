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
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) {
        let mut base: usize = 0;
        let mut next_seq_num = 0;
        let packages: Vec<&[u8]> = message.chunks(BUFFER_SIZE).collect();
        loop {
            if next_seq_num < base + W_SIZE && next_seq_num < packages.len() {
                // let mut pck: [u8] = packages[next_seq_num];
                let mut msg: Vec<u8> = packages[next_seq_num].to_vec();
                // enviando pacotes da janela
                let header = Header {
                    src_addr: self.host,
                    dst_addr: *dst_addr,
                    ack_num: 0,
                    seq_num: next_seq_num as u32,
                    msg_size: msg.len(),
                    checksum: 0,
                    flags: 0,
                    is_last: next_seq_num + 1 == packages.len(),
                    msg: msg,
                };
                self.raw_send(header);
                next_seq_num += 1;
            } else {
                // Lógica para confirmar recebimento de ACKs
                let mut timer = std::time::Instant::now();
                loop {
                    // enquanto não houver timeout nem não receber todos os ACKs
                    if timer.elapsed().as_millis() < TIMEOUT {
                        let mut header = Header::new_empty();
                        // recebendo os acks em um header vazio
                        self.raw_receive(&mut header);
                        if header.ack_num == base as u32 {
                            base += 1;
                        } else {
                            // perdeu um pacote, reinicia a janela a partir do perdido
                            next_seq_num = base;                        
                            break;
                        }
                        if base == next_seq_num {
                            // todos os pacotes foram enviados
                            break;
                        }
                    } else {
                        // timeout estourado
                        next_seq_num = base;
                        break;
                    }
                }
                // se todos os pacotes foram enviados
                if next_seq_num >= packages.len() {
                    break;
                }
            }
        }
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut Vec<u8>) -> (usize, SocketAddr) {
        let mut header = Header::new_empty();
        let mut vec_buffer: Vec<u8> = Vec::new();
        loop {
            self.raw_receive(&mut header);
            vec_buffer.extend_from_slice(&header.msg);
            // Lógica para validar e garantir a entrega confiável
            if self.validate_message(&header) {
                let ack_header = Header {
                    src_addr: self.host,
                    dst_addr: header.src_addr,
                    ack_num: header.seq_num,
                    seq_num: 0,
                    msg_size: 0,
                    checksum: 0,
                    flags: 0,
                    is_last: false,
                    msg: Vec::new(),
                };
                self.raw_send(ack_header);
            }
            if header.is_last {
                break;
            }
        }
        buffer.extend_from_slice(&vec_buffer);
        (vec_buffer.len(), header.src_addr)
    }

    fn validate_message(&self, header: &Header) -> bool {
        // Lógica para validar a mensagem recebida
        true
    }

    fn raw_send(&self, header: Header) {
        self.channel.send(header)
        .expect("Falha ao enviar mensagem no nível Rel_Com\n");
    }

    fn raw_receive(&self, header: &mut Header){
        self.channel.receive(header)
        .expect("Falha ao receber mensagem no nível Rel_Com\n");
    }
}

