/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use crate::channels::Channel;
use crate::packet::Packet;
use logger::log::{Logger, MessageStatus, SenderType};
use logger::{debug_file, debug_println, log};

use std::clone::Clone;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub addr: SocketAddr,
    pub agent_number: usize,
    pub state: NodeState,
}
impl Node {
    pub fn new(addr: SocketAddr, agent_number: usize) -> Self {
        Self {
            addr,
            agent_number,
            state: NodeState::ALIVE,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum NodeState {
    ALIVE,
    //    SUSPECT,
    DEAD,
}

#[derive(PartialEq)]
pub enum Broadcast {
    NONE,
    BEB,
    URB,
    AB,
}

enum SendRequestData {
    // Creates one message to be sent to a specific destination
    Send {
        destination_address: SocketAddr,
    },
    // Creates as many messages as needed to broadcast to the group
    StartBroadcast {},
    // Creates N messages to gossip to neighbors, keeping the original message information
    Gossip {
        origin_address: SocketAddr,
        start_sequence_number: u32,
    },
}

struct SendRequest {
    result_tx: Sender<u32>,
    data: Vec<u8>,
    options: SendRequestData,
}

impl SendRequest {
    pub fn new(data: Vec<u8>, options: SendRequestData) -> (Self, Receiver<u32>) {
        let (result_tx, result_rx) = mpsc::channel();
        (
            Self {
                result_tx,
                data,
                options,
            },
            result_rx,
        )
    }
}

pub struct ReliableCommunication {
    pub host: Node,
    pub group: Mutex<Vec<Node>>,
    leader: Mutex<Node>,
    broadcast: Broadcast,
    timeout: Duration,
    timeout_limit: u32,
    message_timeout: Duration,
    broadcast_timeout: Duration,
    gossip_rate: usize,
    w_size: usize,
    channel: Arc<Channel>,
    register_to_sender_tx: Sender<SendRequest>,
    broadcast_waiters_tx: Sender<Sender<Vec<u8>>>,
    receive_rx: Mutex<Receiver<Vec<u8>>>,
    dst_seq_num_cnt: Mutex<HashMap<SocketAddr, u32>>,
    logger: Logger,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    /// Starts a new thread to listen for any incoming messages
    /// This thread will be responsible for handling the destination of each received packet
    pub fn new(
        host: Node,
        group: Vec<Node>,
        timeout: u64,
        message_timeout: u64,
        timeout_limit: u32,
        w_size: usize,
        gossip_rate: usize,
        broadcast: Broadcast,
        broadcast_timeout: u64,
        logger: Logger,
    ) -> Arc<Self> {
        let channel = Channel::new(host.addr);
        let leader = if broadcast == Broadcast::AB {
            group.first().unwrap().clone()
        } else {
            host.clone()
        };
        let leader = Mutex::new(leader);
        let group = Mutex::new(group);

        let (register_to_sender_tx, register_to_sender_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();

        let (register_to_listener_tx, register_to_listener_rx) = mpsc::channel();
        let (broadcast_waiters_tx, broadcast_waiters_rx) = mpsc::channel();
        let receive_rx = Mutex::new(receive_rx);

        let timeout = Duration::from_millis(timeout);
        let message_timeout = Duration::from_millis(message_timeout);
        let broadcast_timeout = Duration::from_millis(broadcast_timeout);

        let instance = Arc::new(Self {
            host,
            group,
            leader,
            broadcast,
            timeout,
            message_timeout,
            timeout_limit,
            broadcast_timeout,
            gossip_rate,
            w_size,
            channel,
            register_to_sender_tx,
            broadcast_waiters_tx,
            receive_rx,
            dst_seq_num_cnt: Mutex::new(HashMap::new()),
            logger,
        });

        let sender_clone = Arc::clone(&instance);
        let receiver_clone = Arc::clone(&instance);
        let (acks_tx, acks_rx) = mpsc::channel();

        // Spawn all threads
        std::thread::spawn(move || {
            sender_clone.sender(acks_rx, register_to_sender_rx, register_to_listener_tx);
        });
        std::thread::spawn(move || {
            receiver_clone.listener(
                receive_tx,
                acks_tx,
                register_to_listener_rx,
                broadcast_waiters_rx,
            );
        });

        instance
    }

    /// Send a message to a specific destination
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> u32 {
        self.send_nonblocking(dst_addr, message).recv().unwrap()
    }

    fn send_nonblocking(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> Receiver<u32> {
        let (request, result_rx) = SendRequest::new(
            message,
            SendRequestData::Send {
                destination_address: *dst_addr,
            },
        );
        self.register_to_sender_tx.send(request).unwrap();
        result_rx
    }

    /// Read one already received message or wait for a message to arrive
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        match self
            .receive_rx
            .lock()
            .unwrap()
            .recv_timeout(self.message_timeout)
        {
            Ok(msg) => {
                buffer.extend(msg);
                true
            }
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => {
                debug_println!("Erro ao receber mensagem: Canal de comunicação desconectado");
                false
            }
        }
    }

    /// Broadcasts a message, reliability level may be configured in the config file
    pub fn broadcast(&self, message: Vec<u8>) -> u32 {
        match self.broadcast {
            Broadcast::NONE => {
                let group = self.group.lock().unwrap();
                let idx = (self.host.agent_number + 1) % group.len();
                self.send(&group[idx].addr, message) as u32
            }
            Broadcast::BEB => self.beb(message),
            Broadcast::URB => self.urb(message),
            Broadcast::AB => self.ab(message),
        }
    }

    /// Best-Effort Broadcast: attempts to send a message to all nodes in the group and return how many were successful
    /// This algorithm does not garantee delivery to all nodes if the sender fails
    fn beb(&self, message: Vec<u8>) -> u32 {
        let (request, result_rx) = SendRequest::new(message, SendRequestData::StartBroadcast {});
        self.register_to_sender_tx.send(request).unwrap();
        result_rx.recv().unwrap()
    }

    /// Uniform Reliable Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all nodes receive the message if the sender does not fail
    fn urb(&self, message: Vec<u8>) -> u32 {
        let (request, _) = SendRequest::new(message, SendRequestData::StartBroadcast {});
        self.register_to_sender_tx.send(request).unwrap();
        self.group.lock().unwrap().len() as u32
    }

    fn gossip(&self, data: Vec<u8>, origin: SocketAddr, sequence_number: u32) {
        let (request, _) = SendRequest::new(
            data,
            SendRequestData::Gossip {
                origin_address: origin,
                start_sequence_number: sequence_number,
            },
        );
        self.register_to_sender_tx.send(request).unwrap();
    }

    /// Atomic Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all messages are delivered in the same order to all nodes
    fn ab(&self, message: Vec<u8>) -> u32 {
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<Vec<u8>>();
        self.broadcast_waiters_tx.send(broadcast_tx).unwrap();
        loop {
            let (request, request_result_rx) = SendRequest::new (
                message.clone(),
                SendRequestData::StartBroadcast {},
            );
            self.register_to_sender_tx.send(request).unwrap();

            // packet sender logger state
            let logger_state = log::LoggerState::MessageSender { state: MessageStatus::Sent, current_agent_id: self.host.agent_number, target_agent_id: 0, message_id: 0, action: MessageStatus::Waiting, sender_type: SenderType::Unknown };
            //self.logger.log(logger_state);

            // If the chosen leader didn't receive the broadcast request
            // It means it died and we need to pick a new one
            if request_result_rx.recv().unwrap() == 0 {
                let mut ld = self.leader.lock().unwrap();
                let mut group = self.group.lock().unwrap();
                let mut idx = ld.agent_number;
                group[idx].state = NodeState::DEAD;
                while group[idx].state == NodeState::DEAD {
                    idx = (idx + 1) % group.len();
                }
                *ld = group[idx].clone();
                continue;
            }

            // Listen for any broadcasts until your message arrives
            // While there are broadcasts arriving, it means the leader is still alive
            // If the channel times out before your message arrives, it means the leader died
            loop {
                let msg = broadcast_rx.recv_timeout(self.broadcast_timeout);
                match msg {
                    Ok(msg) => {
                        if msg == message {
                            return self.group.lock().unwrap().len() as u32;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        break;
                    }
                    Err(e) => {
                        panic!("{:?}", e)
                    }
                }
            }
        }
    }

    /// Thread to handle the sending of messages
    fn sender(
        self: Arc<Self>,
        acks_rx: Receiver<Packet>,
        register_from_user_rx: Receiver<SendRequest>,
        register_to_listener_tx: Sender<((SocketAddr, SocketAddr), u32)>,
    ) {
        // TODO: Upgrade this thread to make it able of sending multiple messages at once

        // Message sending algorithm
        while let Ok(request) = register_from_user_rx.recv() {
            let messages_to_send = self.get_messages(&request);
            let mut success_count = 0;
            for packets in messages_to_send {
                // Register the destination address and the sequence to the listener thread
                let message_header = packets.first().unwrap().header.clone();
                register_to_listener_tx.send(((message_header.dst_addr, message_header.origin), message_header.seq_num)).unwrap();

                // Go back-N algorithm to send packets
                if self.go_back_n(&packets, &acks_rx) {
                    success_count += 1;
                }
            }

            // Notify the caller that the request was completed
            // The caller may have chosen to not wait for the result, so we ignore if the channel was disconnected
            let _ = request.result_tx.send(success_count);
        }
    }

    fn go_back_n(&self, packets: &Vec<Packet>, acks_rx: &Receiver<Packet>) -> bool {
        let mut base = 0;
        let mut next_seq_num = 0;
        let mut timeout_count = 0;
        let start_packet = packets.first().unwrap().header.seq_num;
        while base < packets.len() {
            // Send window
            while next_seq_num < base + self.w_size && next_seq_num < packets.len() {
                self.channel.send(&packets[next_seq_num]);
                next_seq_num += 1;
            }

            // Wait for an ACK
            // TODO: Somewhere around here: add logic to check if destination is still alive, if not, break the loop, reset the sequence number and return false
            match acks_rx.recv_timeout(self.timeout) {
                Ok(packet) => {
                    // Assume that the listener is sending the number of the highest packet it received
                    // The listener also guarantees that the packet is >= base
                    base = (packet.header.seq_num - start_packet as u32 + 1) as usize;
                },
                Err(RecvTimeoutError::Timeout) => {
                    next_seq_num = base;
                    timeout_count += 1;
                    if timeout_count == self.timeout_limit {
                        debug_println!("Agent {} expecting ack {} upon TIMEOUT LIMIT", self.host.agent_number, start_packet + next_seq_num as u32);
                        return false;
                    }
                },
                Err(RecvTimeoutError::Disconnected) => {
                    debug_println!("Erro ao receber ACKS, Listener thread desconectada");
                    return false;
                },
            }
        }
        true
    }

    fn get_pkts(&self, src_addr: &SocketAddr, dst_addr: &SocketAddr,  origin: &SocketAddr,
        data: Vec<u8>, is_gossip: bool) -> Vec<Packet> {
        let mut seq_lock = self.dst_seq_num_cnt.lock().unwrap();
        let start_seq = seq_lock.entry(*dst_addr).or_insert(0);
        let packets = Packet::packets_from_message(
            *src_addr, *dst_addr, *origin, data, *start_seq, is_gossip,
        );
        *start_seq += packets.len() as u32;
        packets
        }

    fn get_messages(&self, request: &SendRequest) -> Vec<Vec<Packet>> {
        let mut messages = Vec::new();
        match &request.options {
            SendRequestData::Send { destination_address } => {
                let packets = self.get_pkts( &self.host.addr,destination_address, &self.host.addr, request.data.clone(), false);
                messages.push(packets);
            }
            SendRequestData::StartBroadcast {} => match self.broadcast {
                Broadcast::NONE => {
                    debug_println!("Erro: Tentativa de broadcast em um sistema sem broadcast");
                }
                Broadcast::BEB => {
                    for node in self.group.lock().unwrap().iter() {
                            let packets = self.get_pkts(&self.host.addr, &node.addr, &self.host.addr, request.data.clone(), false);
                            messages.push(packets);
                        }
                }
                Broadcast::URB => {
                    let friends = self.get_friends();
                    for node in self.group.lock().unwrap().iter() {
                            let packets = self.get_pkts(&self.host.addr, &node.addr, &self.host.addr, request.data.clone(), true);
                            if friends.contains(node) {
                                messages.push(packets);
                            }
                        }
                    },
                    Broadcast::AB => {
                        let leader = self.group.lock().unwrap()[self.leader.lock().unwrap().agent_number].addr;
                        let packets = self.get_pkts(&self.host.addr, &leader, &self.host.addr, request.data.clone(), true);
                        messages.push(packets);
                }
            },
            SendRequestData::Gossip { origin_address, start_sequence_number } => {
                for node in self.get_friends() {
                    let packets = Packet::packets_from_message(
                        self.host.addr,
                        node.addr,
                        *origin_address,
                        request.data.clone(),
                        *start_sequence_number,
                        true,
                    );
                    messages.push(packets);
                }
            }
        }
        
        messages
    }

    fn get_friends(&self) -> Vec<Node> {
        let group = self.group.lock().unwrap();
        let start = (self.host.agent_number + 1) % group.len();
        let end = (start + self.gossip_rate) % group.len();
        let mut friends = Vec::new();
        if start < end {
            friends.extend_from_slice(&group[start..end]);
        } else {
            friends.extend_from_slice(&group[start..]);
            friends.extend_from_slice(&group[..end]);
        }
        friends
    }

    fn listener(
        self: Arc<Self>,
        messages_tx: Sender<Vec<u8>>,
        acks_tx: Sender<Packet>,
        register_from_sender_rx: Receiver<((SocketAddr, SocketAddr), u32)>,
        register_broadcast_waiters_rx: Receiver<Sender<Vec<u8>>>,
    ) {
        let mut pkts_per_origin: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut expected_acks: HashMap<(SocketAddr, SocketAddr), u32> = HashMap::new();
        let mut broadcast_waiters: Vec<Sender<Vec<u8>>> = Vec::new();
        loop {
            let packet = self.channel.receive();
            if packet.header.is_ack() {
                // Handle ack
                while let Ok((key, start_seq)) = register_from_sender_rx.try_recv() {
                    expected_acks.insert(key, start_seq);
                }
                match expected_acks.get_mut(&(packet.header.src_addr, packet.header.origin)) {
                    Some(seq_num) => {
                        if packet.header.seq_num < *seq_num { continue; }
                        *seq_num = packet.header.seq_num + 1;
                        acks_tx.send(packet).unwrap();
                    }
                    None => {
                        debug_println!("->-> ACK recebido sem destinatário esperando");
                    }
                }
            } else {
                // Handle data
                let packets = pkts_per_origin
                    .entry(packet.header.origin)
                    .or_insert(Vec::new());
                let expected = packets.last().map_or(0, |p| p.header.seq_num + 1);

                // Ignore the packet if the sequence number is higher than expected
                if packet.header.seq_num > expected {
                    continue;
                }
                // Send ack otherwise
                self.channel.send(&packet.get_ack());
                if packet.header.seq_num < expected { continue; }
                
                if packet.header.is_last() {
                    let (message, origin, sequence_number) =
                        ReliableCommunication::receive_last_packet(packets, &packet);
                    // Handling broadcasts
                    if packet.header.must_gossip() {
                        match self.broadcast {
                            // BEB: All broadcasts must be delivered
                            Broadcast::NONE | Broadcast::BEB => {
                                debug_println!("Erro: Pedido de fofoca em um sistema sem fofoca");
                                messages_tx.send(message).unwrap();
                            }
                            // URB: All broadcasts must be gossip   ed and delivered
                            Broadcast::URB => {
                                self.gossip(message.clone(), origin, sequence_number);
                                messages_tx.send(message).unwrap();
                            }
                            // AB: Must check if I'm the leader and should broadcast it or just gossip
                            // In AB, broadcast messages can only be delivered if they were sent by the leader
                            // However, since the leader is the only one that can broadcast,
                            // this is only useful for the leader,
                            // who must not deliver the request to broadcast to the group,
                            // instead, it must broadcast the message and only deliver when it gets gossiped back to it
                            Broadcast::AB => {
                                // Check for anyone waiting for a broadcast
                                // TODO: Remove the broadcast_waiters that are disconnected
                                while let Ok(broadcast_waiter) = register_broadcast_waiters_rx.try_recv() {
                                    broadcast_waiters.push(broadcast_waiter);
                                }
                                for waiter in broadcast_waiters.iter() {
                                    let _ = waiter.send(message.clone());
                                }
                                // Handle the message ????????????????
                                if self.host.agent_number == self.leader.lock().unwrap().agent_number {
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
                            }
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

        for packet in packets.iter() {
            message.extend(&packet.data);
        }
        message.extend(&packet.data);
        let sequence_number = if packets.is_empty() { packet.header.seq_num } else { packets.first().unwrap().header.seq_num };
        packets.clear();
        (message, packet.header.origin, sequence_number)
    }
}
