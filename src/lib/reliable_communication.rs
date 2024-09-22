/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::header::{Packet, HEADER_SIZE};
use crate::config::{BUFFER_SIZE, Node, TIMEOUT, W_SIZE};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub struct ReliableCommunication {
    channel: Channel,
    pub host: SocketAddr,
    pub group: Vec<Node>,
    send_tx: mpsc::Sender<(Sender<Packet>, SocketAddr)>,
    receive_rx: Arc<Mutex<Receiver<Packet>>>,
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
        let mut count_timeout = 0;
        let (ack_tx, ack_rx) = mpsc::channel();
        match self.send_tx.send((ack_tx, self.host)) {
            Ok(_) => {
                let packets: Vec<&[u8]> = message.chunks(BUFFER_SIZE-HEADER_SIZE).collect();
                let start_pkg: usize = {
                    let mut msg_count = self.msg_count.lock().unwrap();
                    let count = msg_count.entry(*dst_addr).or_insert(0);
                    let start = *count;
                    *count += packets.len() as u32;
                    start as usize
                };
                let mut base = 0;
                let mut next_seq_num = 0;
                loop {
                    while next_seq_num < base + W_SIZE && next_seq_num < packets.len() {
                        let msg: Vec<u8> = packets[next_seq_num].to_vec();
                        let packet = Packet::new(
                            self.host,
                            *dst_addr,
                            next_seq_num as u32,
                            if next_seq_num == packets.len() - 1 { 2 } else { 0 },
                            None,
                            msg,
                        );
                        self.raw_send(packet);
                        next_seq_num += 1;
                        // std::thread::sleep(std::time::Duration::from_millis(50));
                    } 
                    match ack_rx.recv_timeout(std::time::Duration::from_millis(TIMEOUT)) {
                        Ok(packet) => {
                            count_timeout = 0;
                            if packet.header.seq_num == (start_pkg + base) as u32 {
                                base += 1;
                                if base == packets.len() {
                                    break;
                                }
                            } else {
                                next_seq_num = base;
                            }
                        },
                        Err(_) => {
                            count_timeout += 1;
                            next_seq_num = base;
                            if count_timeout == 10 {
                                debug_println!("Timeout em Agente {} esperando ACK {}", self.host.port() % 100, start_pkg + base);
                                break;
                            }
                        }
                    }
                }
            },
            Err(_) => {
                let agent = self.host.port() % 100;
                panic!("Erro em Agente {} ao inscrever-se para mandar pacotes", agent);
            }
        }
    }

    fn raw_send(&self, packet: Packet) {
        self.channel.send(packet);
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        let rcv = || {
            match cfg!(debug_assertions) {
                true => self.receive_rx.lock().unwrap().recv_timeout(std::time::Duration::from_millis(10*TIMEOUT)),
                false => self.receive_rx.lock().unwrap().recv().map_err(|_| mpsc::RecvTimeoutError::Timeout)
            }
        };
        loop {
            match rcv() {
                Ok(packet) => {
                    let Packet { data, header, .. } = packet;
                    buffer.extend(data);
                    if header.is_last() { return true; }
                },
                Err(_) => {
                    let agent = self.host.port() % 100;
                   //debug_println!("Agente {} falhou ao receber um pacote", agent);
                    return false;
                }
                
            }
        }
    }
}

