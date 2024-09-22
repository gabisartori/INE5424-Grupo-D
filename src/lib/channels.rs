/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::{UdpSocket, SocketAddr};
use std::io::Error;
use std::process::Output;
use std::sync::mpsc;
use std::{collections, thread};
use std::collections::{HashMap, VecDeque};

use crate::config::BUFFER_SIZE;
use super::header::{Header, Packet, HEADER_SIZE};
use std::sync::{Arc, Mutex};

macro_rules! debug_println {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

// use super::failure_detection;
// Estrutura básica para a camada de comunicação por canais
pub struct Channel {
    socket: UdpSocket,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(
        bind_addr: SocketAddr,
        send_rx: mpsc::Receiver<(mpsc::Sender<Packet>, SocketAddr)>,
        receive_tx: mpsc::Sender<Packet>
        ) -> Result<Self, Error> {
        
        let socket = UdpSocket::bind(bind_addr)?;
        let skt: UdpSocket = match socket.try_clone() {
            Ok(s) => s,
            Err(_) => return Err(Error::new(std::io::ErrorKind::Other, "Erro ao clonar o socket")),
        };
        let agent = bind_addr.port() % 100;
        debug_println!("Incializando Listener no Agente {}", agent);
        thread::spawn(move || Channel::listener(skt, send_rx, receive_tx));
        Ok(Self { socket })
    }
    
    fn listener(socket: UdpSocket, rx_acks: mpsc::Receiver<(mpsc::Sender<Packet>, SocketAddr)>, tx_msgs: mpsc::Sender<Packet>) {
        // a hashmap for the senders, indexed by the destination address
        let mut sends: HashMap<SocketAddr, mpsc::Sender<Packet>> = HashMap::new();
        let mut msgs: HashMap<SocketAddr, VecDeque<Packet>> = HashMap::new();
        loop {
            // Read packets from socket
            let mut buffer = [0; BUFFER_SIZE];
            match socket.recv_from(&mut buffer) {
                Ok((size, src_addr)) => {
                    let packet: Packet = Packet::from_bytes(buffer, size);
                    if packet.is_ack() {
                        Channel::get_txs(&rx_acks, &mut sends);
                        match sends.get(&packet.header.dst_addr) {
                            // Forward the ack to the corresponding sender
                            Some(tx) => {
                                match tx.send(packet.clone()) {
                                    Ok(_) => {
                                        let agent = packet.header.dst_addr.port() % 100;
                                        debug_println!("Delivered ACK {} for Agente {}", packet.header.seq_num, agent);
                                        if packet.is_last() {
                                            // If the message is the last one, remove the sender from the hashmap
                                            let agent = packet.header.dst_addr.port() % 100;
                                            debug_println!("Removing Agente {} from sender subscription", agent);
                                            sends.remove(&packet.header.dst_addr);
                                        }
                                    },
                                    Err(_) => {
                                        let agent = packet.header.dst_addr.port() % 100;
                                        debug_println!("Erro ao entregar ACK {} para o Agente {}", packet.header.seq_num, agent);
                                    },
                                }
                            }
                            // No sender is waiting for this ack, discard it
                            None => {
                                let agent = packet.header.dst_addr.port() % 100;
                                debug_println!("Sender Agent {} not found for ACK {}", agent, packet.header.seq_num);
                            },
                        }
                    } else {
                        // If the packet contains a message, store it in the receivers hashmap and send an ACK
                        let agent: u16 = packet.header.src_addr.port() % 100;
                        debug_println!("Listener read package {} from Agente {} through the socket", packet.header.seq_num, agent);
                        let next_seq_num = match msgs.get(&packet.header.src_addr) {
                            Some(msg) => {
                                let last = msg.back();
                                match last {
                                    Some(last) => {
                                        last.header.seq_num + 1
                                    },
                                    None => 0,
                                }
                            },
                            None => {
                                msgs.insert(packet.header.src_addr, VecDeque::new());
                                0
                            }
                        };

                        if Channel::validate_message(&packet, next_seq_num) {
                            let agent = packet.header.src_addr.port() % 100;
                            debug_println!("Msg was validated, sending ACK {} to Agente {}", packet.header.seq_num, agent);
                            // Send ACK
                            let ack = packet.header.get_ack();
                            match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                                Ok(_) => {
                                    let agent = ack.dst_addr.port() % 100;
                                    debug_println!("Sent ACK {} for Agente {} through the socket sucessfully", ack.seq_num, agent);
                                    msgs.get_mut(&packet.header.src_addr).unwrap().push_back(packet.clone());
                                    let agent = packet.header.src_addr.port() % 100;
                                    if packet.is_last() {
                                        debug_println!("Message {} from Agente {} stored", packet.header.seq_num, agent);
                                        let msg = msgs.entry(packet.header.src_addr).or_insert(VecDeque::new());
                                        if msg[0].is_last() {
                                            msg.pop_front();
                                        }
                                        for pkg in &mut *msg {
                                            Channel::receive(&tx_msgs, pkg);
                                        }
                                        msg.clear();
                                        msg.push_back(packet.clone());
                                    }
                                },
                                Err(_) => {
                                    let agent = ack.dst_addr.port() % 100;
                                    debug_println!("Erro ao enviar ACK {} para o Agente {} pelo socket", ack.seq_num, agent);
                                },
                            }
                        } else {
                            let agent = packet.header.src_addr.port() % 100;
                            debug_println!("Invalid message received from {}", agent);
                        }
                    }
                }
                Err(_) => {
                    debug_println!("Erro ao ler pacote na listener");
                },
            }
        }
    }
    

    fn receive(tx: &mpsc::Sender<Packet>, packet: &Packet) {
        let agent = packet.header.dst_addr.port() % 100;
        debug_println!("Listener is delivering package {} to Agente {}", packet.header.seq_num, agent);
        match tx.send(packet.clone()) {
            Ok(_) => {
                let agent = packet.header.dst_addr.port() % 100;
                debug_println!("Listener delivered package {} to Agente {}", packet.header.seq_num, agent);
            },
            Err(_) => {
                let agent = packet.header.dst_addr.port() % 100;
                debug_println!("Erro ao entregar pacote {} para Agente {}", packet.header.seq_num, agent);
            },
        }
    }

    fn get_txs(rx: &mpsc::Receiver<(mpsc::Sender<Packet>, SocketAddr)>,
               map: &mut HashMap<SocketAddr, mpsc::Sender<Packet>>) {
        debug_println!("Listener is preparing to receive Senders");
        loop {
            match rx.try_recv() {
                Ok((tx, key)) => {
                    match map.get(&key) {
                        Some(_) => {
                            debug_println!("Agente {} is already waiting for a message", key.port() % 100);
                        },
                        None => {
                            let agent = key.port() % 100;
                            debug_println!("Subscribing Agente {} to listener as sender", agent);
                            map.insert(key, tx);
                        }
                    };
                },
                Err(_) => break,
            };
        };
    }

    fn validate_message(packet: &Packet, next_seq_num:u32 ) -> bool {
        let c1: bool = packet.header.checksum == Packet::checksum(&packet.header, &packet.data);
        let c2: bool = packet.header.seq_num == next_seq_num;
        if c1 && c2 {
            println!("Pacote {} validado com sucesso", packet.header.seq_num);
        } else {
            println!("Pacote {} invalidado", packet.header.seq_num);
        }
        c1 && c2
    }

    pub fn send(&self, packet: Packet) { 
        let dst_addr = packet.header.dst_addr;
        let src_addr = packet.header.src_addr;
        let bytes = packet.to_bytes();
        match self.socket.send_to(&bytes, dst_addr) {
            Ok(_) => {
                let agent_d = dst_addr.port() % 100;
                let agent_s = src_addr.port() % 100;
                debug_println!("Sent package {} from Agente {} to Agente {} through socket", packet.header.seq_num, agent_s, agent_d);
            },
            Err(_) => {
                let agent = dst_addr.port() % 100;
                debug_println!("\n---------\nErro ao enviar pacote {} para o Agente {} pelo socket\n--------",
                packet.header.seq_num, agent);
            }
        }
    }
}
