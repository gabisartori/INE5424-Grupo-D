/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::header::Header;
use crate::config::{BUFFER_SIZE, Node, TIMEOUT, W_SIZE};

use std::net::SocketAddr;
use std::sync::mpsc::{self, Sender};

pub struct ReliableCommunication {
    pub channel: Channel,
    pub host: SocketAddr,
    pub group: Vec<Node>,
    pub send_tx: mpsc::Sender<(Sender<Header>, SocketAddr)>,
    pub receive_rx: mpsc::Receiver<Vec<u8>>,
}

// TODO: Fazer com que a inicialização seja de um grupo

impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: SocketAddr, group: Vec<Node>) -> Self {
        let (send_tx, send_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let channel = Channel::new(&host, send_rx, receive_tx).expect("\nFalha ao inicializar o canal no nível Rel_Com\n");
        
        Self { channel, host, group, send_tx, receive_rx}
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) {
        let mut base: usize = 0;
        let mut next_seq_num = 0;
        let (ack_tx, ack_rx) = mpsc::channel();
        let packages: Vec<&[u8]> = message.chunks(BUFFER_SIZE).collect();
        loop {
            while next_seq_num < base + W_SIZE && next_seq_num < packages.len() {
                let msg: Vec<u8> = packages[next_seq_num].to_vec();
                let header = Header {
                    src_addr: self.host,
                    dst_addr: *dst_addr,
                    ack_num: 0,
                    seq_num: next_seq_num as u32,
                    msg_size: msg.len(),
                    checksum: 0,
                    flags: 0,
                    is_last: next_seq_num + 1 == packages.len(),
                    id: 0,
                    msg: msg,
                };
                self.send_tx.send((ack_tx.clone(), *dst_addr)).expect("Falha ao enviar mensagem\n");
                next_seq_num += 1;
            } 
            
            match ack_rx.recv_timeout(std::time::Duration::from_millis(TIMEOUT)) {
                Ok(header) => {
                    if header.ack_num == base as u32 {
                        base += 1;
                        if base == packages.len() {
                            break;
                        }
                    } else {
                        next_seq_num = base;
                    }
                },
                Err(_) => {
                    next_seq_num = base;
                }
            }
        }
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut [u8]) -> (usize, SocketAddr) {
        let char_vec = self.receive_rx.recv().expect("Thread listener terminou e não há mais mensagens para receber\n");
        buffer.copy_from_slice(&char_vec);
        (buffer.len(), self.host)
    }
}

