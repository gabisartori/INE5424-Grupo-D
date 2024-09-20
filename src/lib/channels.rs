/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::{UdpSocket, SocketAddr};
use std::io::Error;
use std::process::Output;
use std::sync::mpsc;
use std::thread;
use std::collections::HashMap;

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
        receive_rx: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>
        ) -> Result<Self, Error> {
        
        let socket = UdpSocket::bind(bind_addr)?;
        let skt: UdpSocket = match socket.try_clone() {
            Ok(s) => s,
            Err(_) => return Err(Error::new(std::io::ErrorKind::Other, "Erro ao clonar o socket")),
        };
        if crate::config::DEBUG {
            println!("Incializando Listener em {}", bind_addr);
        }
        thread::spawn(move || Channel::listener(skt, send_rx, receive_rx));
        Ok(Self { socket })
    }
    
    fn listener(socket: UdpSocket,
                rx_acks: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
                receive_tx: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>) {
        // a hashmap for the senders, indexed by the destination address
        let mut sends: HashMap<SocketAddr, mpsc::Sender<Header>> = HashMap::new();
        let mut receivers: HashMap<SocketAddr, mpsc::Sender<Header>> = HashMap::new();
        let mut msgs: Vec<Header> = Vec::new();
        loop {
            // Read packets from socket
            let mut buffer = [0; BUFFER_SIZE];
            match socket.recv_from(&mut buffer) {
                Ok((size, src_addr)) => {
                    // Get the senders from the channel
                    Channel::get_txs(&rx_acks, &mut sends);
                    Channel::get_txs(&receive_tx, &mut receivers);

                    let header: Header = Header::create_from_bytes(buffer);
                    let source: SocketAddr = header.src_addr;
                    // If packet read is an ACK, send it to the corresponding sender
                    if header.is_ack() { // ack
                        if crate::config::DEBUG {
                            println!("Received ACK from {} with seq_num: {}", source, header.ack_num);
                        }
                        match sends.get(&source) {
                            // Forward the ack to the corresponding sender
                            Some(tx) => {
                                match tx.send(header.clone()) {
                                    Ok(_) => {
                                        if crate::config::DEBUG {
                                            println!("Sent ACK for {} with seq_num: {}", source, header.ack_num);
                                        }
                                    },
                                    Err(_) => {
                                        if crate::config::DEBUG {
                                            println!("\n---------\nErro ao enviar ACK {} para {}\n--------",
                                            header.ack_num, source);
                                        }
                                    },
                                }
                            }
                            // No sender is waiting for this ack, discard it
                            None => {
                                if crate::config::DEBUG {
                                    println!("Sender not found for {}", source);
                                }
                            },
                        }
                    } else {
                        // If the packet contains a message, store it in the receivers hashmap and send an ACK
                        if crate::config::DEBUG {
                            println!("Received message from {} with seq_num: {}", source, header.seq_num);
                        }
                        Channel::prepare_send(&mut receivers, &header, &socket);
                    }
                }
                Err(_) => {
                    if crate::config::DEBUG {
                        println!("\n---------\nErro ao receber mensagem na listener\n---------");
                    }
                },
            }

            // Check if there are messages waiting for the receiver
            for header in msgs.iter() {
                Channel::prepare_send(&mut receivers, header, &socket);
            }
        }
    }
    

    fn get_txs(rx: &mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
               map: &mut HashMap<SocketAddr, mpsc::Sender<Header>> ) {
        
        loop {
            match rx.try_recv() {
                Ok((tx, key)) => {
                    match map.get(&key) {
                        Some(_) => continue,
                        None => {
                            if crate::config::DEBUG {
                                println!("Subscribing {} to listener", key);
                            }
                            map.insert(key, tx)}
                    };
                },
                Err(_) => break,
            };
        };
    }

    fn prepare_send(receivers: &mut HashMap<SocketAddr, mpsc::Sender<Header>>,
                    header: &Header, socket: &UdpSocket) {
        match receivers.get_mut(&header.dst_addr) {
            Some(tx) => {
                // Send the message to the receiver
                match tx.send(header.clone()) {
                    Ok(_) => {
                        if crate::config::DEBUG {
                            println!("Sending message to {} with seq_num: {}",
                            header.dst_addr, header.seq_num);
                        }
                    },
                    Err(_) => {
                        if crate::config::DEBUG {
                            println!("\n---------\nErro ao enviar mensagem com seq_num {} para {}\n--------",
                            header.seq_num, header.dst_addr);
                        }
                    },
                }
                // Send ACK
                let ack = header.get_ack();
                if crate::config::DEBUG {
                    println!("Sending ACK for {} with seq_num: {}", ack.dst_addr, ack.ack_num);
                }
                match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                    Ok(_) => {
                        if crate::config::DEBUG {
                            println!("Sent ACK for {} with seq_num: {}", ack.dst_addr, ack.ack_num);
                        }
                    },
                    Err(_) => {
                        if crate::config::DEBUG {
                            println!("\n---------\nErro ao enviar ACK {} para {}\n--------",
                            ack.ack_num, ack.dst_addr);
                        }
                    },
                }
                if header.is_last {
                    // If the message is the last one, remove the receiver from the hashmap
                    if crate::config::DEBUG {
                        println!("Removing receiver {}", header.dst_addr);
                    }
                    receivers.remove(&header.dst_addr);
                }
            }
            None => if crate::config::DEBUG {
                println!("Receiver not found for {}", header.dst_addr);
            },
        }
    }

    pub fn send(&self, header: Header) {
        let dst_addr = header.dst_addr;
        let bytes = header.to_bytes();
        match self.socket.send_to(&bytes, dst_addr) {
            Ok(_) => (),
            Err(_) => {
                println!("\n---------\nErro ao enviar mensagem de seq_num {} para {}\n--------",
                        header.seq_num, dst_addr)}
        }
    }
}
