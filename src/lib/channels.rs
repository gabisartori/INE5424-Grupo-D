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
        bind_addr: &SocketAddr,
        send_rx: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
        receive_tx: mpsc::Sender<Vec<u8>>
        ) -> Result<Self, Error> {
        
        let socket = UdpSocket::bind(bind_addr)?;
        let skt: UdpSocket = socket.try_clone().unwrap();
        thread::spawn(move || Channel::listener(skt, send_rx, receive_tx));
        Ok(Self { socket })
    }

    fn listener(socket: UdpSocket, rx_acks: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>, receive_tx: mpsc::Sender<Vec<u8>>) {
        // a hashmap for the senders, indexed by the destination address
        let mut sends: HashMap<SocketAddr, mpsc::Sender<Header>> = HashMap::new();
        let mut receivers: HashMap<(SocketAddr, u32), Vec<u8>> = HashMap::new();
        loop {
            loop {
                // Receber tx sempre que a função send for chamada
                match rx_acks.try_recv() {
                    // Se a função send foi chamada, armazenar o tx (unwraped)
                    Ok((tx, key)) => sends.insert(key, tx),
                    // Se não, quebrar o loop
                    Err(_) => break,
                };
            }
            // Read packets from socket
            let mut buffer = [0; BUFFER_SIZE + HEADER_SIZE];
            match socket.recv_from(&mut buffer) {
                Ok((size, src_addr)) => {
                    let header = Header::pure_from_bytes(buffer);
                    let dst = header.dst_addr;
                    let msg_id = header.id;
                    // If packet read is an ACK, send it to the corresponding sender
                    if header.flags == 1 { // ack
                        match sends.get(&dst) {
                            // Forward the ack to the corresponding sender
                            Some(tx) => {
                                match tx.send(header.clone()) {
                                    Ok(_) => (),
                                    Err(_) => (),
                                }
                            }
                            // No sender is waiting for this ack, discard it
                            None => (),
                        }
                    } else {
                        // If the packet contains a message, store it in the receivers hashmap and send an ACK
                        if !receivers.contains_key(&(dst, msg_id)) {
                            receivers.insert((dst, msg_id), Vec::new());
                        }
                        match receivers.get_mut(&(dst, msg_id)) {
                            Some(buff) => {
                                let ack = header.get_ack();
                                buff.extend_from_slice(&header.msg);
                                // DEBUG
                                let msg = std::str::from_utf8(&header.msg).unwrap();
                                println!("Received message from {}:\n{}", header.src_addr, msg);
                                // Send ACK
                                match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                                    Ok(_) => (),
                                    Err(_) => (),
                                }
                            }
                            None => (),
                        }
                        if header.is_last {
                            match receivers.remove(&(dst, msg_id)) {
                                Some(msg) => {
                                    receive_tx.send(msg).unwrap();
                                }
                                None => (),
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    }
}
