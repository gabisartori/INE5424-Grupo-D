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
        let skt: UdpSocket = socket.try_clone().unwrap();
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
            let mut buffer = [0; BUFFER_SIZE + HEADER_SIZE];
            match socket.recv_from(&mut buffer) {

                Ok((size, src_addr)) => {
                    // Get the senders from the channel
                    Channel::get_txs(&rx_acks, &mut sends);
                    Channel::get_txs(&receive_tx, &mut receivers);

                    let header: Header = Header::create_from_bytes(buffer);
                    let dst: SocketAddr = header.dst_addr;
                    // If packet read is an ACK, send it to the corresponding sender
                    if header.is_ack() { // ack
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
                        Channel::prepare_send(&mut receivers, &header, &socket);
                    }
                }
                Err(_) => continue,
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
                        None => map.insert(key, tx)
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
                // DEBUG
                let message = std::str::from_utf8(&header.msg).unwrap();
                println!("Received message from {}:\n{}", header.src_addr, message);
                match tx.send(header.clone()) {
                    Ok(_) => (),
                    Err(_) => (),
                }
                // Send ACK
                let ack = header.get_ack();
                match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                    Ok(_) => (),
                    Err(_) => (),
                }
                if header.is_last {
                    // If the message is the last one, remove the receiver from the hashmap
                    receivers.remove(&header.dst_addr);
                }
            }
            None => (),
        }
    }

    pub fn send(&self, header: Header) {
        let dst_addr = header.dst_addr;
        let bytes = header.to_bytes();
        self.socket.send_to(&bytes, dst_addr).unwrap();
    }
}
