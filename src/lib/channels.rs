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
    tx: mpsc::Sender<(mpsc::Sender<Header>, SocketAddr)>,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(bind_addr: &SocketAddr,
        input_rx: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>)
         -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        // Instantiate sender and listener threads
        // And create a channel to communicate between them
        // MPSC: Multi-Producer Single-Consumer
        let skt: UdpSocket = socket.try_clone().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || Channel::listener(input_rx, rx, skt));
        Ok(Self { socket, tx })
    }

    fn listener(rx_acks: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
                rx_msgs: mpsc::Receiver<(mpsc::Sender<Header>, SocketAddr)>,
                socket: UdpSocket) {
        let mut msgs: Vec<Header> = Vec::new();
        // a hashmap for the senders, indexed by the destination address
        let mut sends: HashMap<SocketAddr, mpsc::Sender<Header>> = HashMap::new();
        let mut receivers: HashMap<SocketAddr, mpsc::Sender<Header>> = HashMap::new();
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
            loop {
                // Receber tx sempre que a função send for chamada
                match rx_msgs.try_recv() {
                    // Se a função send foi chamada, armazenar o tx (unwraped)
                    Ok((tx, key)) => receivers.insert(key, tx),
                    // Se não, quebrar o loop
                    Err(_) => break,
                };
            }
            // Read packets from socket
            let mut buffer = [0; BUFFER_SIZE + HEADER_SIZE];
            match socket.recv_from(&mut buffer) {
                Ok((size, src_addr)) => {
                    let mut header = Header::new_empty();
                    header.from_bytes(buffer);
                    // If it's an ACK, send it to the corresponding sender
                    if header.flags == 1 { // ack
                        let dst = header.dst_addr;
                        match sends.get(&dst) {
                            Some(tx) => {
                                match tx.send(header.clone()) {
                                    Ok(_) => (),
                                    Err(_) => (),
                                }
                            }
                            None => (),
                        }
                    } else {
                        // If it's a message, keep it to itself and send an ACK
                        let dst = header.dst_addr;
                        match receivers.get(&dst) {
                            Some(tx) => {
                                match tx.send(header.clone()) {
                                    Ok(_) => (),
                                    Err(_) => (),
                                }
                                let ack = header.get_ack();
                                msgs.push(header.clone());
                                let msg = std::str::from_utf8(&header.msg).unwrap();
                                println!("Received message from {}:\n{}", header.src_addr, msg);
                                match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                                    Ok(_) => (),
                                    Err(_) => (),
                                }
                            }
                            None => (),
                        }
                        
                    }
                }
                Err(_) => continue,
            }
        }
    }

    pub fn receive(&self, header: &mut Header) {
        // If there's a message ready, return it
        // Else: block and wait for there to be a message
        return;
    }
}
