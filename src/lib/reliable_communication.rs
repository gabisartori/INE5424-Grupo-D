/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::packet::{Packet, HEADER_SIZE};
use crate::config::{Node, Broadcast, BROADCAST, BUFFER_SIZE, TIMEOUT, MESSAGE_TIMEOUT, W_SIZE};

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, Arc};
use std::time::Duration;

pub struct ReliableCommunication {
    pub host: Node,
    pub group: Vec<Node>,
    channel: Arc<Channel>,
    register_to_sender_tx: Sender<(Sender<bool>, Vec<u8>, SocketAddr)>,
    receive_rx: Mutex<Receiver<Vec<u8>>>,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    /// Starts a new thread to listen for any incoming messages
    /// This thread will be responsible for handling the destination of each received packet
    pub fn new(host: Node, group: Vec<Node>) -> Arc<Self> {
        let (register_to_sender_tx, register_to_sender_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let (acks_tx, acks_rx) = mpsc::channel();
        let (register_to_listener_tx, register_to_listener_rx) = mpsc::channel();
        let (channel_tx, channel_rx) = mpsc::channel();
        
        let socket = Arc::new(std::net::UdpSocket::bind(host.addr).unwrap());
        
        let channel = Arc::new(Channel::new(socket.clone(), channel_tx));
        let socket_reader = Arc::clone(&channel);
        let receive_rx = Mutex::new(receive_rx);
        
        let instance = Arc::new(Self { host, group, channel, register_to_sender_tx, receive_rx });
        let receiver_clone = Arc::clone(&instance);
        // Spawn all threads
        socket_reader.run();
        std::thread::spawn(move || {
            ReliableCommunication::sender(socket, acks_rx, register_to_sender_rx, register_to_listener_tx);
        });
        std::thread::spawn(move || {
            receiver_clone.listener(channel_rx, receive_tx, acks_tx, register_to_listener_rx);
        });

        instance
    }

    /// Send a message to a specific destination
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> bool {
        let (result_tx, result_rx) = mpsc::channel();
        self.register_to_sender_tx.send((result_tx, message, *dst_addr)).unwrap();
        result_rx.recv().unwrap()
    }

    /// Read one already received message or wait for a message to arrive
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        match self.receive_rx.lock().unwrap().recv_timeout(Duration::from_millis(MESSAGE_TIMEOUT)) {
            Ok(msg) => {
                buffer.extend(msg);
                true
            },
            Err(_) => false,
        }
    }

    /// Broadcasts a message, reliability level may be configured in the config file
    pub fn broadcast(&self, message: Vec<u8>) -> u32 {
        match BROADCAST {
            Broadcast::NONE => {
                let idx = (self.host.agent_number + 1) as usize % self.group.len();
                self.send(&self.group[idx].addr, message) as u32
            },
            Broadcast::BEB => self.beb(message, &self.group).len() as u32,
            Broadcast::URB => self.urb(message),
            Broadcast::AB => self.ab(message),
        }
    }

    /// Best-Effort Broadcast: attempts to send a message to all nodes in the group and return how many were successful
    /// This algorithm does not garantee delivery to all nodes if the sender fails
    fn beb(&self, message: Vec<u8>, group: &Vec<Node>) -> Vec<Node> {
        group.iter()
            .filter(|node| {
                self.send(&node.addr, message.clone())
            })
            .cloned()
            .collect()
    }

    /// Uniform Reliable Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all nodes receive the message if the sender does not fail
    fn urb(&self, message: Vec<u8>) -> u32 {
        self.beb(message, &self.group).len() as u32
    }

    /// Atomic Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all messages are delivered in the same order to all nodes
    fn ab(&self, message: Vec<u8>) -> u32 {
        self.urb(message)
    }

    /// Thread to handle the sending of messages
    fn sender(socket: Arc<UdpSocket>, acks_rx: Receiver<Packet>, register_from_user_rx: Receiver<(Sender<bool>, Vec<u8>, SocketAddr)>, register_to_listener_tx: Sender<(SocketAddr, u32)>) {
        // TODO: Upgrade this thread to make it able of sending multiple messages at once
        let mut destination_sequence_number_counter: HashMap<SocketAddr, usize> = HashMap::new();
        let mut base;
        let mut next_seq_num;
        // Message sending algorithm
        while let Ok((result_tx, message, dst_addr)) = register_from_user_rx.recv() {
            // Fragment message into packets
            let chunks: Vec<&[u8]> = message.chunks(BUFFER_SIZE - HEADER_SIZE).collect();
            let start_packet = *destination_sequence_number_counter.entry(dst_addr).or_insert(0);
            destination_sequence_number_counter.insert(dst_addr, start_packet + chunks.len());
            
            let packets: Vec<Packet> = chunks.iter().enumerate().map(|(i, chunk)| {
                Packet::new(
                    socket.local_addr().unwrap(),
                    dst_addr,
                    (start_packet + i) as u32,
                    None,
                    i == (chunks.len() - 1),
                    false,
                    false,
                    false,
                    chunk.to_vec(),
                )
            }).collect();

            // Register the destination address and the sequence to the listener thread
            register_to_listener_tx.send((dst_addr, packets.first().unwrap().header.seq_num)).unwrap();

            // Go back-N algorithm to send packets
            base = 0;
            next_seq_num = 0;
            let destination = packets[0].header.dst_addr;
            while base < packets.len() {
                // Send window
                while next_seq_num < base + W_SIZE && next_seq_num < packets.len() {
                    socket.send_to(&packets[next_seq_num].to_bytes(), destination.clone()).unwrap();
                    next_seq_num += 1;
                }

                // Wait for an ACK
                // TODO: Somewhere around here: add logic to check if destination is still alive, if not, break the loop, reset the sequence number and return false
                match acks_rx.recv_timeout(Duration::from_millis(TIMEOUT)) {
                    Ok(packet) => {
                        // Assume that the listener is sending the number of the highest packet it received
                        // The listener also guarantees that the packet is >= base
                        base = (packet.header.seq_num - start_packet as u32 + 1) as usize;
                    },
                    Err(RecvTimeoutError::Timeout) => {
                        next_seq_num = base;
                    },
                    Err(RecvTimeoutError::Disconnected) => {
                        debug_println!("Erro ao enviar pacote: Canal de comunicação desconectado");
                        result_tx.send(false).unwrap();
                        return;
                    },
                }
            }

            // Return the result of the operation to the caller
            result_tx.send(true).unwrap();
        }
    }

    fn listener(self: Arc<Self>, channel_rx: Receiver<Packet>, messages_tx: Sender<Vec<u8>>, acks_tx: Sender<Packet>, register_from_sender_rx: Receiver<(SocketAddr, u32)>) {
        let mut packets_per_source: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut expected_acks: HashMap<SocketAddr, u32> = HashMap::new();
        while let Ok(packet) = channel_rx.recv() {
            if packet.header.is_ack() {
                // Handle ack
                while let Ok((key, start_seq)) = register_from_sender_rx.try_recv() {
                    match expected_acks.entry(key) {
                        Entry::Occupied(mut entry) => {
                            let seq_num = entry.get_mut();
                            *seq_num = start_seq;
                        },
                        Entry::Vacant(entry) => {
                            entry.insert(start_seq);
                        }
                    }    
                }
                match expected_acks.get_mut(&packet.header.src_addr){
                    Some(seq_num) => {
                        if packet.header.seq_num < *seq_num { continue; }
                        *seq_num = packet.header.seq_num + 1;
                        acks_tx.send(packet).unwrap();
                    }
                    None => {
                        debug_println!("->-> ACK recebido sem destinatário esperando");
                    },
                }
            } else {
                // Handle data
                let packets = packets_per_source.entry(packet.header.src_addr).or_insert(Vec::new());
                let expected = packets.last().map_or(0, |p| p.header.seq_num + 1);
        
                // Ignore the packet if the sequence number is higher than expected
                if packet.header.seq_num > expected { continue; }
                // Send ack otherwise
                self.channel.send(&packet.get_ack());
                if packet.header.seq_num < expected { continue; }
                
                if packet.header.is_last() {
                    let mut message = Vec::new();
                    if !packets.is_empty() && packets.first().expect("Vetor de pacotes está vazio").header.is_last() { packets.remove(0); }
                    for packet in packets.iter() {
                        message.extend(&packet.data);
                    }
                    message.extend(&packet.data);
                    messages_tx.send(message).unwrap();
                    packets.clear();
                }
                packets.push(packet);
            }
        }
    }
}

