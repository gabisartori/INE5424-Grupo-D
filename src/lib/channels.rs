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
        receive_tx: mpsc::Sender<Header>
        ) -> Result<Self, Error> {
        
        let socket = UdpSocket::bind(bind_addr)?;
        let skt: UdpSocket = match socket.try_clone() {
            Ok(s) => s,
            Err(_) => return Err(Error::new(std::io::ErrorKind::Other, "Erro ao clonar o socket")),
        };
        if crate::config::DEBUG {
            let agent = bind_addr.to_string()[bind_addr.to_string().len()-4..].to_string();
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
        // let mut receivers: HashMap<SocketAddr, mpsc::Sender<Header> > = HashMap::new();
        // let mut msgs: Vec<Header> = Vec::new();
        loop {
            // Read packets from socket
            let mut buffer = [0; BUFFER_SIZE];
            match socket.recv_from(&mut buffer) {
                Ok((size, src_addr)) => {
                    // Get the senders from the channel
                    Channel::get_txs(&rx_acks, &mut sends, "Sender");

                    let header: Header = Header::create_from_bytes(buffer);
                    // If packet read is an ACK, send it to the corresponding sender
                    if header.is_ack() { // ack
                        if crate::config::DEBUG {
                            let agent = header.src_addr.to_string()[header.src_addr.to_string().len()-4..].to_string();
                            println!("Listener read ACK {} from Agente {} through the socket", header.ack_num, agent);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        match sends.get(&header.dst_addr) {
                            // Forward the ack to the corresponding sender
                            Some(tx) => {
                                match tx.send(header.clone()) {
                                    Ok(_) => {
                                        if crate::config::DEBUG {
                                            let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                                            println!("Delivered ACK {} for Agente {}", header.ack_num, agent);
                                            let _ = std::io::Write::flush(&mut std::io::stdout());
                                        }
                                        if header.is_last {
                                            // If the message is the last one, remove the sender from the hashmap
                                            if crate::config::DEBUG {
                                                let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                                                println!("Removing Agente {} from sender subscription", agent);
                                                let _ = std::io::Write::flush(&mut std::io::stdout());
                                            }
                                            sends.remove(&header.dst_addr);
                                        }
                                    },
                                    Err(_) => {
                                        if crate::config::DEBUG {
                                            let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                                            println!("\n---------\nErro ao entregar ACK {} para o Agente {}\n--------",
                                            header.ack_num, agent);
                                            let _ = std::io::Write::flush(&mut std::io::stdout());
                                        }
                                    },
                                }
                            }
                            // No sender is waiting for this ack, discard it
                            None => {
                                if crate::config::DEBUG {
                                    let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                                    println!("Sender Agent {} not found for ACK {}", agent, header.ack_num);
                                    let _ = std::io::Write::flush(&mut std::io::stdout());
                                }
                            },
                        }
                    } else {
                        // If the packet contains a message, store it in the receivers hashmap and send an ACK
                        if crate::config::DEBUG {
                            let agent = header.src_addr.to_string()[header.src_addr.to_string().len()-4..].to_string();
                            println!("Listener read package {} from Agente {} through the socket", header.seq_num, agent);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        if Channel::validate_message(&header) {
                            // Send ACK
                            let ack = header.get_ack();
                            if crate::config::DEBUG {
                                let agent = ack.dst_addr.to_string()[ack.dst_addr.to_string().len()-4..].to_string();
                                println!("Msg was validated, sending ACK {} to Agente {}",
                                ack.ack_num, agent);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                                Ok(_) => {
                                    if crate::config::DEBUG {
                                        let agent = ack.dst_addr.to_string()[ack.dst_addr.to_string().len()-4..].to_string();
                                        println!("Sent ACK {} for Agente {} through the socket sucessfully", ack.ack_num, agent);
                                        let _ = std::io::Write::flush(&mut std::io::stdout());
                                    }
                                    // msgs.push(header.clone());
                                    match tx_msgs.send(header.clone()) {
                                        Ok(_) => {
                                            if crate::config::DEBUG {
                                                let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                                                println!("Delivered package {} to Agente {}", header.seq_num, agent);
                                                let _ = std::io::Write::flush(&mut std::io::stdout());
                                            }
                                        },
                                        Err(_) => {
                                            if crate::config::DEBUG {
                                                let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                                                println!("\n---------\nErro ao entregar pacote {} para Agente {}\n--------",
                                                header.seq_num, agent);
                                                let _ = std::io::Write::flush(&mut std::io::stdout());
                                            }
                                        },
                                    }
                                },
                                Err(_) => {
                                    if crate::config::DEBUG {
                                        let agent = ack.dst_addr.to_string()[ack.dst_addr.to_string().len()-4..].to_string();
                                        println!("\n---------\nErro ao enviar ACK {} para o Agente {} pelo socket\n--------",
                                        ack.ack_num, agent);
                                        let _ = std::io::Write::flush(&mut std::io::stdout());
                                    }
                                },
                            }
                        } else if crate::config::DEBUG {
                            let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                            println!("Invalid message received from {}", agent);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                    }
                }
                Err(_) => {
                    if crate::config::DEBUG {
                        println!("\n---------\nErro ao ler pacote na listener\n---------");
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }
                },
            }

            // Check if there are messages waiting for the receiver
            // let mut to_keep: Vec<Header> = Vec::new();
            // for header in msgs {
            //     if !Channel::prepare_receive(&mut receivers, &header) {
            //         to_keep.push(header);
            //     }
            // }
            // msgs = to_keep;
        }
    }
    

    fn prepare_receive(receivers: &mut HashMap<SocketAddr, mpsc::Sender<Header>>,
                    header: &Header) -> bool {
        match receivers.get_mut(&header.dst_addr) {
            Some(tx) => {
                // Send the message to the receiver
                match tx.send(header.clone()) {
                    Ok(_) => {
                        if crate::config::DEBUG {
                            let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                            println!("Listener delivered package {} to Agente {}", header.seq_num, agent);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        if header.is_last {
                            // If the message is the last one, remove the receiver from the hashmap
                            if crate::config::DEBUG {
                                let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                                println!("Removing Agente {} from receivers subscription", agent);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            receivers.remove(&header.dst_addr);
                        }
                        true
                    },
                    Err(_) => {
                        if crate::config::DEBUG {
                            let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                            println!("\n---------\nErro ao entregar pacote {} para Agente {}\n--------",
                            header.seq_num, agent);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        false
                    },
                }
            }
            None => {
                if crate::config::DEBUG {
                    let agent = header.dst_addr.to_string()[header.dst_addr.to_string().len()-4..].to_string();
                    println!("Agente {} not found as Receiver", agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                false
            },
        }
    }

    fn get_txs(rx: &mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
               map: &mut HashMap<SocketAddr, mpsc::Sender<Header>>,
               role: &str) {
        
        loop {
            match rx.try_recv() {
                Ok((tx, key)) => {
                    match map.get(&key) {
                        Some(_) => {
                            println!("Agente {} is already waiting for a message", key);
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        },
                        None => {
                            if crate::config::DEBUG {
                                let agent = key.to_string()[key.to_string().len()-4..].to_string();
                                println!("Subscribing Agente {} to listener as {}", agent, role);
                                let _ = std::io::Write::flush(&mut std::io::stdout());
                            }
                            map.insert(key, tx);}
                    };
                },
                Err(_) => break,
            };
        };
    }

    fn validate_message(header: &Header) -> bool {
        // let c1: bool = header.checksum == Header::get_checksum(&header.msg);
        // c1
        let n = rand::random::<u8>();
        n % 10 == 0
    }

    pub fn send(&self, header: Header) { 
        let dst_addr = header.dst_addr;
        let src_addr = header.src_addr;
        let bytes = header.to_bytes();
        match self.socket.send_to(&bytes, dst_addr) {
            Ok(_) => {
                if crate::config::DEBUG {
                    let agent_d = dst_addr.to_string()[dst_addr.to_string().len()-4..].to_string();
                    let agent_s = src_addr.to_string()[src_addr.to_string().len()-4..].to_string();
                    println!("Agente {} sent package {} to Agente {} through socket sucessfully", agent_s, header.seq_num, agent_d);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
            },
            Err(_) => {
                if crate::config::DEBUG {
                    let agent = dst_addr.to_string()[dst_addr.to_string().len()-4..].to_string();
                    println!("\n---------\nErro ao enviar pacote {} para o Agente {} pelo socket\n--------",
                        header.seq_num, agent);
                    let _ = std::io::Write::flush(&mut std::io::stdout());}
                }
        }
    }
}
