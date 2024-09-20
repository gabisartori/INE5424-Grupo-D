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

        if crate::config::DEBUG {
            // get 4 last characters from self.host
            let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
            println!("Agente {agent} is subscribing to listener to send packages");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        match self.send_tx.send((ack_tx, self.host)) {
            Ok(_) => {
                if crate::config::DEBUG {
                    let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                    println!("Agente {} sent subscription to send packages sucessfully",
                    agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
            },
            Err(_) => {
                if crate::config::DEBUG {
                    let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                    println!("\n---------\nErro em Agente {} ao inscrever-se para mandar pacotes\n--------", agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
            }
        }
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
                self.raw_send(header);
                next_seq_num += 1;
            } 
            if crate::config::DEBUG {
                // recovers the last character from self.host
                let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                println!("Agente {} is waiting for ACK {}",agent, base);
                let _ = std::io::Write::flush(&mut std::io::stdout());                
            }
            match ack_rx.recv_timeout(std::time::Duration::from_millis(TIMEOUT)) {
                Ok(header) => {
                    if crate::config::DEBUG {
                        let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                        println!("Agente {} received ACK {}", agent, header.ack_num);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }
                    if header.ack_num == base as u32 {
                        base += 1;
                        if base == packages.len() {
                            break;
                        }
                    } else {
                        if crate::config::DEBUG {
                            let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                            println!("Agente {} expected ACK {} but received ACK {}",
                            agent, base, header.ack_num);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        next_seq_num = base;
                    }
                },
                Err(_) => {
                    if crate::config::DEBUG {
                        let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                        println!("Timeout for Agente {}, resending from {} to {}",
                            agent, base, next_seq_num);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }
                    next_seq_num = base;
                }
            }
        }
    }

    fn raw_send(&self, header: Header) {
        if crate::config::DEBUG {
            let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
            println!("Agente {} is sending package {}",
            agent, header.seq_num);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        self.channel.send(header);
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut Vec<u8>) {
        let (msg_tx, msg_rx) = mpsc::channel();
        let mut next_seq_num = 0;
        if crate::config::DEBUG {
            let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
            println!("Agente {} is subscribing to listener to receive messages",
                    agent);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        match self.receive_tx.send((msg_tx.clone(), self.host)) {
            Ok(_) => {
                let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                if crate::config::DEBUG {
                    println!("Agente {} sent subscription to receive messages", agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                loop {
                    match msg_rx.recv() {
                        Ok(header) => {
                            if crate::config::DEBUG {
                                let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                                println!("Agente {} received package {}", agent, header.seq_num);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            if header.seq_num == next_seq_num {
                                buffer.extend(header.msg);
                                next_seq_num += 1;
                                if header.is_last {
                                    break;
                                }
                            } // listener already sends ack
                        },
                        Err(_) => {
                            if crate::config::DEBUG {
                                let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                                println!("\n---------\nAgente {} falhou ao receber o pacote {}\n--------",
                                agent, next_seq_num);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                        }
                        
                    }
                }
            }
            Err(_) => if crate::config::DEBUG {
                let agent = self.host.to_string()[self.host.to_string().len()-4..].to_string();
                println!("\n---------\nErro no Agente {} ao inscrever-se como recebedor\n--------", agent);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
        }
    }
}

