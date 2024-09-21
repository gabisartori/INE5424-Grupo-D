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
use super::header::{Header, HEADER_SIZE};
use std::sync::{Arc, Mutex};
// use super::failure_detection;
// Estrutura básica para a camada de comunicação por canais
pub struct Channel {
    socket: UdpSocket,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(
        bind_addr: SocketAddr,
        send_rx: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
        receive_tx: mpsc::Sender<Header>
        ) -> Result<Self, Error> {
        
        let socket = UdpSocket::bind(bind_addr)?;
        let skt: UdpSocket = match socket.try_clone() {
            Ok(s) => s,
            Err(_) => return Err(Error::new(std::io::ErrorKind::Other, "Erro ao clonar o socket")),
        };
        if cfg!(debug_assertions) {
            let agent = bind_addr.port() % 100;
            println!("Incializando Listener no Agente {}", agent);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        thread::spawn(move || Channel::listener(skt, send_rx, receive_tx));
        Ok(Self { socket })
    }
    
    fn listener(socket: UdpSocket,
                rx_acks: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
                tx_msgs: mpsc::Sender<Header>) {
        // a hashmap for the senders, indexed by the destination address
        let mut sends: HashMap<SocketAddr, mpsc::Sender<Header>> = HashMap::new();
        let mut msgs: HashMap<SocketAddr, VecDeque<Header>> = HashMap::new();
        loop {
            // Read packets from socket
            let mut buffer = [0; BUFFER_SIZE];
            match socket.recv_from(&mut buffer) {
                Ok((size, src_addr)) => {
                    let header: Header = Header::create_from_bytes(buffer, size);
                    // If packet read is an ACK, send it to the corresponding sender
                    if header.is_ack() { // ack
                        if cfg!(debug_assertions) {
                            let agent = header.src_addr.port() % 100;
                            println!("Listener read ACK {} from Agente {} through the socket", header.ack_num, agent);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        // Get the senders from the channel
                        Channel::get_txs(&rx_acks, &mut sends);
                        match sends.get(&header.dst_addr) {
                            // Forward the ack to the corresponding sender
                            Some(tx) => {
                                match tx.send(header.clone()) {
                                    Ok(_) => {
                                        if cfg!(debug_assertions) {
                                            let agent = header.dst_addr.port() % 100;
                                            println!("Delivered ACK {} for Agente {}", header.ack_num, agent);
                                            let _ = std::io::Write::flush(&mut std::io::stdout());
                                        }
                                        if header.is_last {
                                            // If the message is the last one, remove the sender from the hashmap
                                            if cfg!(debug_assertions) {
                                                let agent = header.dst_addr.port() % 100;
                                                println!("Removing Agente {} from sender subscription", agent);
                                                let _ = std::io::Write::flush(&mut std::io::stdout());
                                            }
                                            sends.remove(&header.dst_addr);
                                        }
                                    },
                                    Err(_) => {
                                        if cfg!(debug_assertions) {
                                            let agent = header.dst_addr.port() % 100;
                                            println!("\n---------\nErro ao entregar ACK {} para o Agente {}\n--------",
                                            header.ack_num, agent);
                                            let _ = std::io::Write::flush(&mut std::io::stdout());
                                        }
                                    },
                                }
                            }
                            // No sender is waiting for this ack, discard it
                            None => {
                                if cfg!(debug_assertions) {
                                    let agent = header.dst_addr.port() % 100;
                                    println!("Sender Agent {} not found for ACK {}", agent, header.ack_num);
                                    let _ = std::io::Write::flush(&mut std::io::stdout());
                                }
                            },
                        }
                    } else {
                        // If the packet contains a message, store it in the receivers hashmap and send an ACK
                        if cfg!(debug_assertions) {
                            let agent: u16 = header.src_addr.port() % 100;
                            println!("Listener read package {} from Agente {} through the socket", header.seq_num, agent);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        let next_seq_num = match msgs.get(&header.src_addr) {
                            Some(msg) => {
                                let last = msg.back();
                                match last {
                                    Some(last) => {
                                        last.seq_num + 1
                                    },
                                    None => 0,
                                }
                            },
                            None => {
                                msgs.insert(header.src_addr, VecDeque::new());
                                0
                            }
                        };

                        if Channel::validate_message(&header, next_seq_num) {
                            if cfg!(debug_assertions) {
                                let agent = header.src_addr.port() % 100;
                                println!("Msg was validated, sending ACK {} to Agente {}",
                                header.seq_num, agent);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            // Send ACK
                            let ack = header.get_ack();
                            match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                                Ok(_) => {
                                    if cfg!(debug_assertions) {
                                        let agent = ack.dst_addr.port() % 100;
                                        println!("Sent ACK {} for Agente {} through the socket sucessfully", ack.ack_num, agent);
                                        let _ = std::io::Write::flush(&mut std::io::stdout());
                                    }
                                    msgs.get_mut(&header.src_addr).unwrap().push_back(header.clone());
                                    if header.is_last {
                                        if cfg!(debug_assertions) {
                                            let agent = header.src_addr.port() % 100;
                                            println!("Sending complete message from {} ", agent);
                                            let _ = std::io::Write::flush(&mut std::io::stdout());
                                        }
                                        let msg = msgs.entry(header.src_addr).or_insert(VecDeque::new());
                                        if msg[0].is_last {
                                            msg.pop_front();
                                        }
                                        for pkg in &mut *msg {
                                            Channel::receive(&tx_msgs, pkg);
                                        }
                                        msg.clear();
                                        msg.push_back(header.clone());
                                    }
                                },
                                Err(_) => {
                                    if cfg!(debug_assertions) {
                                        let agent = ack.dst_addr.port() % 100;
                                        println!("\n---------\nErro ao enviar ACK {} para o Agente {} pelo socket\n--------",
                                        ack.ack_num, agent);
                                        let _ = std::io::Write::flush(&mut std::io::stdout());
                                    }
                                },
                            }
                        } else if cfg!(debug_assertions) {
                            let agent = header.dst_addr.port() % 100;
                            println!("Invalid message received from {}", agent);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                    }
                }
                Err(_) => {
                    if cfg!(debug_assertions) {
                        println!("\n---------\nErro ao ler pacote na listener\n---------");
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }
                },
            }
        }
    }
    

    fn receive(tx: &mpsc::Sender<Header>, header: &Header) {
        if cfg!(debug_assertions) {
            let agent = header.dst_addr.port() % 100;
            println!("Listener is delivering package {} to Agente {}", header.seq_num, agent);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        match tx.send(header.clone()) {
            Ok(_) => {
                if cfg!(debug_assertions) {
                    let agent = header.dst_addr.port() % 100;
                    println!("Listener delivered package {} to Agente {}", header.seq_num, agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }                
            },
            Err(_) => {
                if cfg!(debug_assertions) {
                    let agent = header.dst_addr.port() % 100;
                    println!("\n---------\nErro ao entregar pacote {} para Agente {}\n--------",
                    header.seq_num, agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
            },
        }
    }

    fn get_txs(rx: &mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
               map: &mut HashMap<SocketAddr, mpsc::Sender<Header>>) {
        if cfg!(debug_assertions) {
            println!("Listener is preparing to receive Senders");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }        
        loop {
            match rx.try_recv() {
                Ok((tx, key)) => {
                    match map.get(&key) {
                        Some(_) => {
                            if cfg!(debug_assertions) {
                                println!("Agente {} is already waiting for a message", key);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                        },
                        None => {
                            if cfg!(debug_assertions) {
                                let agent = key.port() % 100;
                                println!("Subscribing Agente {} to listener as sender", agent);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            map.insert(key, tx);
                        }
                    };
                },
                Err(_) => break,
            };
        };
    }

    fn validate_message(header: &Header, next_seq_num:u32 ) -> bool {
        let c1: bool = header.checksum == Header::get_checksum(&header.msg);
        let c2: bool = header.seq_num == next_seq_num;
        c1 && c2
    }

    pub fn send(&self, header: Header) { 
        let dst_addr = header.dst_addr;
        let src_addr = header.src_addr;
        let bytes = header.to_bytes();
        match self.socket.send_to(&bytes, dst_addr) {
            Ok(_) => {
                if cfg!(debug_assertions) {
                    let agent_d = dst_addr.port() % 100;
                    let agent_s = src_addr.port() % 100;
                    println!("Agente {} sent package {} to Agente {} through socket sucessfully", agent_s, header.seq_num, agent_d);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
            },
            Err(_) => {
                if cfg!(debug_assertions) {
                    let agent = dst_addr.port() % 100;
                    println!("\n---------\nErro ao enviar pacote {} para o Agente {} pelo socket\n--------",
                        header.seq_num, agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());}
                }
        }
    }
}
