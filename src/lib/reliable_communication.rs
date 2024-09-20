/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::header::{Header, HEADER_SIZE};
use crate::config::{BUFFER_SIZE, Node, TIMEOUT, W_SIZE};

use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, Sender};

pub struct ReliableCommunication {
    pub channel: Channel,
    pub host: SocketAddr,
    pub group: Vec<Node>,
    pub send_tx: mpsc::Sender<(Sender<Header>, SocketAddr)>,
    pub receive_tx: mpsc::Sender<(Sender<Header>, SocketAddr)>,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: SocketAddr, group: Vec<Node>) -> Self {
        let (send_tx, send_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let channel = match Channel::new(host.clone(), send_rx, receive_rx) {
            Ok(c) => c,
            Err(_) => panic!("Erro ao criar o canal de comunicação"),
        };
        
        Self { channel, host, group, send_tx, receive_tx}
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) {
        let mut base: usize = 0;
        let mut next_seq_num = 0;
        let (ack_tx, ack_rx) = mpsc::channel();
        let packages: Vec<&[u8]> = message.chunks(BUFFER_SIZE-HEADER_SIZE).collect();
        loop {
            while next_seq_num < base + W_SIZE && next_seq_num < packages.len() {
                let msg: Vec<u8> = packages[next_seq_num].to_vec();
                let header = Header::new(
                    self.host,
                    *dst_addr,
                    0,
                    next_seq_num as u32,
                    msg.len(),
                    0,
                    next_seq_num + 1 == packages.len(),
                    msg,
                );
                self.raw_send(ack_tx.clone(), header);
                next_seq_num += 1;
            } 
            if crate::config::DEBUG {
                println!("{} is waiting for acks {}",
                            self.host, base);
            }
            match ack_rx.recv_timeout(std::time::Duration::from_millis(TIMEOUT)) {
                Ok(header) => {
                    if crate::config::DEBUG {
                        println!("{} received ack with ack_num: {}",
                                self.host, header.ack_num);
                    }
                    if header.ack_num == base as u32 {
                        base += 1;
                        if base == packages.len() {
                            break;
                        }
                    } else {
                        if crate::config::DEBUG {
                            println!("{} received ack with ack_num: {} but expected {}",
                            self.host, header.ack_num, base);
                        }
                        next_seq_num = base;
                    }
                },
                Err(_) => {
                    if crate::config::DEBUG {
                        println!("Timeout em {}, resending from {} to {}", self.host, base, next_seq_num);
                    }
                    next_seq_num = base;
                }
            }
        }
    }

    fn raw_send(&self, ack_tx: Sender<Header>, header: Header) {
        if crate::config::DEBUG {
            println!("{} is subscribing to listener to send a msg", self.host);
        }
        match self.send_tx.send((ack_tx, header.dst_addr)) {
            Ok(_) => {},
            Err(_) => {
                if crate::config::DEBUG {
                    println!("\n---------\nErro em {} ao enviar mensagem\n--------",
                            self.host);
                }
            }
        }
        if crate::config::DEBUG {
            println!("{} is sending message with seq_num: {}", self.host, header.seq_num);
        }
        self.channel.send(header);
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut Vec<u8>) -> (usize, SocketAddr) {
        let (msg_tx, msg_rx) = mpsc::channel();
        let mut next_seq_num = 0;
        let mut sender: SocketAddr;
        loop {
            if crate::config::DEBUG {
                println!("{} is subscribing to listener to receive a msg", self.host);
            }
            match self.receive_tx.send((msg_tx.clone(), self.host)) {
                Ok(_) => {
                    if crate::config::DEBUG {
                        println!("{} is waiting for message with seq_num: {}",
                        self.host, next_seq_num);
                    }
                    match msg_rx.recv() {
                        Ok(header) => {
                            if header.seq_num == next_seq_num {
                                buffer.extend(header.msg);
                                next_seq_num += 1;
                                sender = header.src_addr;
                                if header.is_last {
                                    break;
                                }
                            } // listener already sends ack
                        },
                        Err(_) => {
                            if crate::config::DEBUG {
                                println!("\n---------\nErro em {} ao enviar mensagem\n--------",
                                self.host);
                            }
                        }
                        
                    }
                },
                Err(_) => if crate::config::DEBUG {
                    println!("\n---------\nErro em {} ao enviar mensagem\n--------",
                            self.host);
                }
            }
        }
        (buffer.len(), sender) 
    }
}

