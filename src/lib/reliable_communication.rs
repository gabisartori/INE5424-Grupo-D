/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::header::{Header, HEADER_SIZE};
use crate::config::{BUFFER_SIZE, Node, TIMEOUT, W_SIZE};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub struct ReliableCommunication {
    channel: Channel,
    pub host: SocketAddr,
    pub group: Vec<Node>,
    send_tx: mpsc::Sender<(Sender<Header>, SocketAddr)>,
    receive_rx: Arc<Mutex<Receiver<Header>>>,
    // uma variável compartilhada (Arc<Mutex>) que conta quantas vezes send foi chamada
    msg_count: Arc<Mutex<HashMap<SocketAddr, u32>>>,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: SocketAddr, group: Vec<Node>) -> Self {
        let (send_tx, send_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let channel = match Channel::new(host.clone(), send_rx, receive_tx) {
            Ok(c) => c,
            Err(_) => panic!("Erro ao criar o canal de comunicação"),
        };
        let receive_rx = Arc::new(Mutex::new(receive_rx));
        
        Self { channel, host, group, send_tx, receive_rx, msg_count: Arc::new(Mutex::new(HashMap::new())) }
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) {
        if cfg!(debug_assertions) {
            // get 4 last characters from self.host
            //let agent = self.host.to_string()[self.host.port() % 100;
            let agent = self.host.port() % 100;
            println!("Agente {agent} is subscribing to listener to send packages");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        let (ack_tx, ack_rx) = mpsc::channel();
        match self.send_tx.send((ack_tx, self.host)) {
            Ok(_) => {
                if cfg!(debug_assertions) {
                    let agent = self.host.port() % 100;
                    println!("Agente {} sent subscription to send packages sucessfully",
                    agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                let packages: Vec<&[u8]> = message.chunks(BUFFER_SIZE-HEADER_SIZE).collect();
                let mut start_pkg: usize = {
                    let mut msg_count = self.msg_count.lock().unwrap();
                    let count = msg_count.entry(*dst_addr).or_insert(0);
                    let start = *count;
                    *count += packages.len() as u32;
                    start as usize
                };
                let mut base = start_pkg;
                let mut next_seq_num = base;
                loop {
                    while next_seq_num < base + W_SIZE && next_seq_num - start_pkg < packages.len() {
                        let msg: Vec<u8> = packages[next_seq_num - start_pkg].to_vec();
                        let header = Header::new(
                            self.host,
                            *dst_addr,
                            0,
                            next_seq_num as u32,
                            packages.len(),
                            0,
                            next_seq_num - start_pkg == packages.len() - 1,
                            msg,
                        );
                        self.raw_send(header);
                        next_seq_num += 1;
                    } 
                    if cfg!(debug_assertions) {
                        // recovers the last character from self.host
                        let agent = self.host.port() % 100;
                        println!("Agente {} is waiting for ACK {}",agent, base);
                        let _ = std::io::Write::flush(&mut std::io::stdout());                
                    }
                    match ack_rx.recv_timeout(std::time::Duration::from_millis(TIMEOUT)) {
                        Ok(header) => {
                            if cfg!(debug_assertions) {
                                let agent = self.host.port() % 100;
                                println!("Agente {} received ACK {}", agent, header.ack_num);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            if header.ack_num == base as u32 {
                                base += 1;
                                if base - start_pkg == packages.len() {
                                    break;
                                }
                            } else {
                                if cfg!(debug_assertions) {
                                    let agent = self.host.port() % 100;
                                    println!("Agente {} expected ACK {} but received ACK {}",
                                    agent, base, header.ack_num);
                                    let _ = std::io::Write::flush(&mut std::io::stdout());
                                }
                                next_seq_num = base;
                            }
                        },
                        Err(_) => {
                            if cfg!(debug_assertions) {
                                let agent = self.host.port() % 100;
                                println!("Timeout for Agente {}, resending from {} to {}",
                                    agent, base, next_seq_num);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            next_seq_num = base;
                        }
                    }
                }
            },
            Err(_) => {
                let agent = self.host.port() % 100;
                panic!("\n---------\nErro em Agente {} ao inscrever-se para mandar pacotes\n--------", agent);
            }
        }
    }

    fn raw_send(&self, header: Header) {
        if cfg!(debug_assertions) {
            let agent = self.host.port() % 100;
            println!("Agente {} is sending package {}",
            agent, header.seq_num);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        self.channel.send(header);
    }
    

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        let mut next_seq_num = 0;
        if cfg!(debug_assertions) {
            let agent = self.host.port() % 100;
            println!("Agente {} is preparing to receive messages from listener", agent);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        let rcv = || {
            match cfg!(debug_assertions) {
                true => self.receive_rx.lock().unwrap().recv_timeout(std::time::Duration::from_millis(10*TIMEOUT)),
                false => self.receive_rx.lock().unwrap().recv().map_err(|_| mpsc::RecvTimeoutError::Timeout)
            }
        };
        loop {
            match rcv() {
                Ok(header) => {
                    if cfg!(debug_assertions) {
                        let agent = self.host.port() % 100;
                        println!("Agente {} received package {}", agent, header.seq_num);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }
                    buffer.extend(header.msg);
                    if header.is_last {
                        return true;
                    } // listener already sends ack
                },
                Err(_) => {
                    if cfg!(debug_assertions) {
                        let agent = self.host.port() % 100;
                        println!("\n---------\nAgente {} falhou ao receber o pacote {}\nThread Listener terminou\n--------",
                        agent, next_seq_num);
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                        return false;
                    }
                }
                
            }
        }
    }
}

