/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::packet::Packet;

use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, Arc};
use std::time::Duration;
use std::clone::Clone;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub addr: SocketAddr,
    pub agent_number: usize
}
struct SendRequest {
    result_tx: Sender<u32>,
    destination_address: Option<SocketAddr>,
    origin_address: Option<SocketAddr>,
    start_sequence_number: Option<u32>,
    is_broadcast: bool,
    data: Vec<u8>,
}
#[derive(PartialEq)]
pub enum Broadcast {
    NONE,
    BEB,
    URB,
    AB
}

pub struct ReliableCommunication {
    pub host: Node,
    pub group: Vec<Node>,
    leader: Mutex<usize>,
    broadcast: Broadcast,
    timeout: u64,
    message_timeout: u64,
    broadcast_timeout: u64,
    gossip_rate: usize,
    w_size: usize,
    channel: Arc<Channel>,
    register_to_sender_tx: Sender<SendRequest>,
    broadcast_waiters_tx: Sender<Sender<Vec<u8>>>,
    receive_rx: Mutex<Receiver<Vec<u8>>>,
    dst_seq_num_cnt: Mutex<HashMap<SocketAddr, u32>>,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    /// Starts a new thread to listen for any incoming messages
    /// This thread will be responsible for handling the destination of each received packet
    pub fn new(host: Node, group: Vec<Node>, timeout: u64,
    message_timeout: u64, w_size: usize, gossip_rate: usize,
    broadcast: Broadcast, broadcast_timeout: u64) -> Arc<Self> {
        let (register_to_sender_tx, register_to_sender_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let (acks_tx, acks_rx) = mpsc::channel();
        let (register_to_listener_tx, register_to_listener_rx) = mpsc::channel();
        let (broadcast_waiters_tx, broadcast_waiters_rx) = mpsc::channel();
        let receive_rx = Mutex::new(receive_rx);                
        let channel = Channel::new(host.addr);
        
        let leader = if broadcast == Broadcast::AB {group.first().unwrap().agent_number} else {host.agent_number};
        let leader = Mutex::new(leader);
        let instance = Arc::new(Self {
            host, group, leader, broadcast, timeout, message_timeout, broadcast_timeout,
            gossip_rate, w_size, channel, register_to_sender_tx, broadcast_waiters_tx,
            receive_rx, dst_seq_num_cnt: Mutex::new(HashMap::new()) });
        let sender_clone = Arc::clone(&instance);
        let receiver_clone = Arc::clone(&instance);
        // Spawn all threads
        std::thread::spawn(move || {
            sender_clone.sender(acks_rx, register_to_sender_rx, register_to_listener_tx);
        });
        std::thread::spawn(move || {
            receiver_clone.listener(receive_tx, acks_tx, register_to_listener_rx, broadcast_waiters_rx);
        });

        instance
    }

    /// Send a message to a specific destination
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> u32 {
        self.send_nonblocking(dst_addr, message).recv().unwrap()
    }

    fn send_nonblocking(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> Receiver<u32> {
        let (result_tx, result_rx) = mpsc::channel();
        let request = SendRequest {
            result_tx,
            destination_address: Some(*dst_addr),
            origin_address: None,
            start_sequence_number: None,
            is_broadcast: false,
            data: message,
        };
        self.register_to_sender_tx.send(request).unwrap();
        result_rx
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
        match self.broadcast {
            Broadcast::NONE => {
                let idx = (self.host.agent_number + 1) % self.group.len();
                self.send(&self.group[idx].addr, message) as u32
            },
            Broadcast::BEB => self.beb(message),
            Broadcast::URB => self.urb(message),
            Broadcast::AB => self.ab(message),
        }
    }

    /// Best-Effort Broadcast: attempts to send a message to all nodes in the group and return how many were successful
    /// This algorithm does not garantee delivery to all nodes if the sender fails
    fn beb(&self, message: Vec<u8>) -> u32 {
        //debug_println!("{} BEB to {:?}", self.host.agent_number, group);
        let (result_tx, result_rx) = mpsc::channel();
        let request = SendRequest {
            result_tx,
            destination_address: None,
            origin_address: None,
            start_sequence_number: None,
            is_broadcast: true,
            data: message,
        };
        self.register_to_sender_tx.send(request).unwrap();
        result_rx.recv().unwrap()
    }

    /// Uniform Reliable Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all nodes receive the message if the sender does not fail
    fn urb(&self, message: Vec<u8>) -> u32 {
        self.beb(message)
    }

    fn gossip(&self, message: Vec<u8>, origin: SocketAddr, sequence_number: u32) -> bool {
        let request = SendRequest {
            result_tx: mpsc::channel().0,
            destination_address: None,
            origin_address: Some(origin),
            start_sequence_number: Some(sequence_number),
            is_broadcast: true,
            data: message,
        };
        self.register_to_sender_tx.send(request).unwrap();
        true
    }

    /// Atomic Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all messages are delivered in the same order to all nodes
    fn ab(&self, message: Vec<u8>) -> u32 {
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<Vec<u8>>();
        self.broadcast_waiters_tx.send(broadcast_tx).unwrap();
        loop {
            let (request_result_tx, request_result_rx) = mpsc::channel();
            let leader = Some(self.group[*self.leader.lock().unwrap()].addr);
            let request = SendRequest {
                result_tx: request_result_tx,
                destination_address: leader,
                origin_address: None,
                start_sequence_number: None,
                is_broadcast: true,
                data: message.clone(),
            };
            self.register_to_sender_tx.send(request).unwrap();
            // If the chosen leader didn't receive the broadcast request
            // It means it died and we need to pick a new one
            if request_result_rx.recv().unwrap() == 0 { continue; }

            // Listen for any broadcasts until your message arrives
            // While there are broadcasts arriving, it means the leader is still alive
            // If the channel times out before your message arrives, it means the leader died
            loop {
                let msg = broadcast_rx
                .recv_timeout(Duration::from_millis(self.broadcast_timeout));
                match msg {
                    Ok(msg) => {
                        if msg == message {
                            return self.group.len() as u32;
                        }
                    },
                    Err(RecvTimeoutError::Timeout) => {
                        break;
                    },
                    Err(e) => {
                        panic!("{:?}", e)
                    },
                }
            }

        }
    }

    /// Thread to handle the sending of messages
    fn sender(
        self: Arc<Self>,
        acks_rx: Receiver<Packet>,
        register_from_user_rx: Receiver<SendRequest>,
        register_to_listener_tx: Sender<(SocketAddr, u32)>
    ) {
        // TODO: Upgrade this thread to make it able of sending multiple messages at once
        let mut messages_to_send: VecDeque<Vec<Packet>> = VecDeque::new();
        
        // Message sending algorithm
        while let Ok(request) = register_from_user_rx.recv() {
            self.handle_request(&mut messages_to_send, &request);
            let result_tx = request.result_tx;
            let mut success_count = 0;
            while let Some(packets) = messages_to_send.pop_front() {
                // Register the destination address and the sequence to the listener thread
                let ugh = packets.first().unwrap().header.clone();
                register_to_listener_tx.send((ugh.dst_addr, ugh.seq_num)).unwrap();

                debug_println!("Agente {} reencaminhando mensagem {} do {} para {}", self.host.agent_number, ugh.seq_num, ugh.origin, ugh.dst_addr);

                // Go back-N algorithm to send packets
                if self.go_back_n(&packets, &acks_rx) {
                    success_count += 1;
                }
            }

            // Notify the caller that the request was completed
            // The caller may have chosen to not wait for the result, so we ignore if the channel was disconnected
            let _ = result_tx.send(success_count);
        }
    }

    fn handle_request(&self, messages_to_send: &mut VecDeque<Vec<Packet>>, request: &SendRequest) {
        if request.is_broadcast {
            match self.broadcast {
                Broadcast::NONE => {debug_println!("Erro: Tentativa de broadcast em um sistema sem broadcast");},
                Broadcast::BEB => {
                    for node in self.group.iter() {
                        let packets = self.get_packets(request.data.clone(), node.addr, None, false, None);
                        messages_to_send.push_back(packets);
                    }
                },
                Broadcast::URB => {
                    let friends = self.get_friends();
                    for node in self.group.iter() {
                        let packets = self.get_packets(request.data.clone(), node.addr, request.origin_address, true, request.start_sequence_number);
                        if friends.contains(node) {
                            messages_to_send.push_back(packets);
                        }
                    }
                },
                Broadcast::AB => {
                    let leader = self.group[*self.leader.lock().unwrap()].addr;
                    let packets = self.get_packets(request.data.clone(), leader, request.origin_address, true, request.start_sequence_number);
                    messages_to_send.push_back(packets);
                }
            }
        } else {
            let destination = request.destination_address.unwrap();
            let packets = self.get_packets(request.data.clone(), destination, None, false, None);
            messages_to_send.push_back(packets);
        }
    }

    fn go_back_n(
        &self,
        packets: &Vec<Packet>,
        acks_rx: &Receiver<Packet>
    ) -> bool {
        let mut base = 0;
        let mut next_seq_num = 0;
        let start_packet = packets.first().unwrap().header.seq_num;
        while base < packets.len() {
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
                    debug_println!("Erro ao receber ACKS, Listener thread desconectada");
                    return false;
                },
            }
        }
        true
    }

    fn get_packets(&self, data: Vec<u8>, destination: SocketAddr,
        origin: Option<SocketAddr>, is_gossip: bool, sq: Option<u32>) -> Vec<Packet> {
        let mut destination_seq = self.dst_seq_num_cnt.lock().unwrap();
        let start_seq;
        if sq.is_none() { start_seq = *destination_seq.entry(destination).or_insert(0); }
        else { start_seq = sq.unwrap(); }
        
        let packets = Packet::packets_from_message(
            self.host.addr,
            destination,
            origin.unwrap_or(self.host.addr),
            data,
            start_seq,
            is_gossip,
        );
        if !origin.is_none() {
            match destination_seq.get_mut(&destination) {
                Some(dst) => {
                    *dst += packets.len() as u32;
                }
                None => {
                    destination_seq.insert(destination, packets.len() as u32);
                }
            }
        }
        packets
    }

    fn get_friends(&self) -> Vec<Node> {
        let start = (self.host.agent_number + 1) % self.group.len();
        let end = (start + self.gossip_rate) % self.group.len();
        let mut friends = Vec::new();
        if start < end {
            friends.extend_from_slice(&self.group[start..end]);
        } else {
            friends.extend_from_slice(&self.group[start..]);
            friends.extend_from_slice(&self.group[..end]);
        }
        friends
    }

    fn listener(
        self: Arc<Self>,
        messages_tx: Sender<Vec<u8>>,
        acks_tx: Sender<Packet>,
        register_from_sender_rx: Receiver<(SocketAddr, u32)>,
        register_broadcast_waiters_rx: Receiver<Sender<Vec<u8>>>
    ) {
        let mut pkts_per_origin: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut expected_acks: HashMap<SocketAddr, u32> = HashMap::new();
        let mut broadcast_waiters: Vec<Sender<Vec<u8>>> = Vec::new();
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
                let packets = pkts_per_origin.entry(packet.header.origin).or_insert(Vec::new());
                let expected = packets.last().map_or(0, |p| p.header.seq_num + 1);

                // Ignore the packet if the sequence number is higher than expected
                if packet.header.seq_num > expected { continue; }
                // Send ack otherwise
                self.channel.send(&packet.get_ack());
                if packet.header.seq_num < expected { continue; }
                
                if packet.header.is_last() {
                    let (message, origin, sequence_number) = ReliableCommunication::receive_last_packet(packets, &packet);
                    // Handling broadcasts
                    if packet.header.must_gossip() {
                        match self.broadcast {
                            // BEB: All broadcasts must be delivered
                            Broadcast::NONE | Broadcast::BEB => {
                                messages_tx.send(message).unwrap();
                            },
                            // URB: All broadcasts must be gossiped and delivered
                            Broadcast::URB => {
                                self.gossip(message.clone(), origin, sequence_number);
                                messages_tx.send(message).unwrap();
                            },
                            // AB: Must check if I'm the leader and should broadcast it or just gossip
                            // In AB, broadcast messages can only be delivered if they were sent by the leader
                            // However since the leader is the only one that can broadcast, this is only useful for the leader,
                            // Who must not deliver the request to broadcast to the group, instead, it must broadcast the message and only deliver when it gets gossiped back to it
                            Broadcast::AB => {
                                // Check for anyone waiting for a broadcast
                                // TODO: Remove the broadcast_waiters that are disconnected
                                while let Ok(broadcast_waiter) = register_broadcast_waiters_rx.try_recv() {
                                    broadcast_waiters.push(broadcast_waiter);
                                }
                                for waiter in broadcast_waiters.iter() {
                                    let _ =  waiter.send(message.clone());
                                }
                                // Handle the message
                                if self.host.agent_number == *self.leader.lock().unwrap() {
                                    if packet.header.origin == self.host.addr {
                                        self.gossip(message.clone(), origin, sequence_number);
                                        messages_tx.send(message).unwrap();
                                    } else {
                                        self.urb(message.clone());  
                                    }
                                } else {
                                    self.gossip(message.clone(), origin, sequence_number);
                                    messages_tx.send(message).unwrap();
                                }
                            },
                        }
                    } else {
                        messages_tx.send(message).unwrap();
                    }
                }
                packets.push(packet);
            }
        }
    }

    fn receive_last_packet(packets: &mut Vec<Packet>, packet: &Packet) -> (Vec<u8>, SocketAddr, u32) {
        let mut message = Vec::new();
        // Ignore the first packet if its the remnant of a previous message
        if !packets.is_empty() && packets.first().unwrap().header.is_last() { packets.remove(0); }
        let sequence_number = packets.first().unwrap().header.seq_num;
        for packet in packets.iter() {
            message.extend(&packet.data);
        }
        packets.clear();
        message.extend(&packet.data);
        (message, packet.header.origin, sequence_number)
    }
}

