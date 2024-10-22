/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::packet::{Packet, HEADER_SIZE};
use crate::config::{Broadcast, BROADCAST, BUFFER_SIZE};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, Arc};
use std::time::Duration;
use std::clone::Clone;

#[derive(Clone)]
#[derive(Debug)]
pub struct Node {
    pub addr: SocketAddr,
    pub agent_number: usize
}

pub struct ReliableCommunication {
    pub host: Node,
    pub group: Vec<Node>,
    timeout: u64,
    message_timeout: u64,
    w_size: usize,
    channel: Arc<Channel>,
    register_to_sender_tx: Sender<(Sender<bool>, Vec<Packet>, SocketAddr)>,
    receive_rx: Mutex<Receiver<Vec<u8>>>,
    dst_seq_num_cnt: Mutex<HashMap<SocketAddr, u32>>,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    pub const W_SIZE: usize = 5;
    pub const TIMEOUT: u64 = 1;
    pub const MESSAGE_TIMEOUT: u64 = 1000;
    /// Starts a new thread to listen for any incoming messages
    /// This thread will be responsible for handling the destination of each received packet
    pub fn new(host: Node, group: Vec<Node>) -> Arc<Self> {
        let (register_to_sender_tx, register_to_sender_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let (acks_tx, acks_rx) = mpsc::channel();
        let (register_to_listener_tx, register_to_listener_rx) = mpsc::channel();
                
        let channel = Channel::new(host.addr);
        let receive_rx = Mutex::new(receive_rx);
        
        let instance = Arc::new(Self {
            host, group, timeout: ReliableCommunication::TIMEOUT,
            message_timeout: ReliableCommunication::MESSAGE_TIMEOUT,
            w_size:ReliableCommunication::W_SIZE,
            channel, register_to_sender_tx,
            receive_rx, dst_seq_num_cnt: Mutex::new(HashMap::new()) });
        let sender_clone = Arc::clone(&instance);
        let receiver_clone = Arc::clone(&instance);
        // Spawn all threads
        std::thread::spawn(move || {
            sender_clone.sender(acks_rx, register_to_sender_rx, register_to_listener_tx);
        });
        std::thread::spawn(move || {
            receiver_clone.listener(receive_tx, acks_tx, register_to_listener_rx);
        });

        instance
    }

    /// Send a message to a specific destination
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> bool {
        let packets = self.get_packets(dst_addr, message, false);
        self.send_pkts(dst_addr, packets)
    }
    /// Fragment message into packets
    fn get_packets(&self, dst_addr: &SocketAddr,
        message: Vec<u8>, gossip: bool) -> Vec<Packet> {
        let chunks: Vec<&[u8]> = message.chunks(BUFFER_SIZE - HEADER_SIZE).collect();
        let mut dst_seq_num_cnt = self.dst_seq_num_cnt.lock().unwrap();
        let start_packet = *dst_seq_num_cnt.entry(*dst_addr).or_insert(0);
        dst_seq_num_cnt.insert(*dst_addr, start_packet + chunks.len() as u32);
        
        chunks.iter().enumerate().map(|(i, chunk)| {
            Packet::new(
                self.channel.socket_address(),
                *dst_addr,
                start_packet + i as u32,
                None,
                i == (chunks.len() - 1),
                false,
                gossip,
                chunk.to_vec(),
            )
        }).collect()
    }

    fn send_pkts(&self, dst_addr: &SocketAddr, packets: Vec<Packet>) -> bool {
        let (result_tx, result_rx) = mpsc::channel();
        self.register_to_sender_tx.send((result_tx, packets, *dst_addr)).unwrap();
        result_rx.recv().unwrap()
    }

    /// Read one already received message or wait for a message to arrive
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        match self.receive_rx.lock().unwrap().recv_timeout(Duration::from_millis(self.message_timeout)) {
            Ok(msg) => {
                buffer.extend(msg);
                true
            },
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => {
                debug_println!("Erro ao receber mensagem: Canal de comunicação desconectado");
                false
            },
        }
    }

    /// Broadcasts a message, reliability level may be configured in the config file
    pub fn broadcast(&self, message: Vec<u8>) -> u32 {
        match BROADCAST {
            Broadcast::NONE => {
                let idx = (self.host.agent_number + 1) % self.group.len();
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
    fn sender(self: Arc<Self>, acks_rx: Receiver<Packet>,
        register_from_user_rx: Receiver<(Sender<bool>, Vec<Packet>, SocketAddr)>,
        register_to_listener_tx: Sender<(SocketAddr, u32)>) {
        // TODO: Upgrade this thread to make it able of sending multiple messages at once
        let mut base;
        let mut next_seq_num;
        // Message sending algorithm
        while let Ok((result_tx, packets, dst_addr)) = register_from_user_rx.recv() {
            let start_packet = packets.first().expect("Vetor de pacotes está vazio").header.seq_num;

            // Register the destination address and the sequence to the listener thread
            register_to_listener_tx.send((dst_addr, packets.first().unwrap().header.seq_num)).unwrap();

            // Go back-N algorithm to send packets
            base = 0;
            next_seq_num = 0;
            'message_loop: while base < packets.len() {
                // Send window
                while next_seq_num < base + self.w_size && next_seq_num < packets.len() {
                    self.channel.send(&packets[next_seq_num]);
                    next_seq_num += 1;
                }

                // Wait for an ACK
                // TODO: Somewhere around here: add logic to check if destination is still alive, if not, break the loop, reset the sequence number and return false
                match acks_rx.recv_timeout(Duration::from_millis(self.timeout)) {
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
                        // continues the message_loop loop
                        continue 'message_loop;

                    },
                }
            }

            // Return the result of the operation to the caller
            result_tx.send(true).unwrap();
        }
    }

    fn listener(self: Arc<Self>, messages_tx: Sender<Vec<u8>>, acks_tx: Sender<Packet>, register_from_sender_rx: Receiver<(SocketAddr, u32)>) {
        let mut packets_per_source: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut expected_acks: HashMap<SocketAddr, u32> = HashMap::new();
        loop {
            let packet = self.channel.receive();
            if packet.header.is_ack() {
                // Handle ack
                while let Ok((key, start_seq)) = register_from_sender_rx.try_recv() {
                    expected_acks.entry(key).or_insert(start_seq);
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
                    if !packets.is_empty() && packets.first().unwrap().header.is_last() { packets.remove(0); }
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

