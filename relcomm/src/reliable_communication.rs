/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

use crate::channels::Channel;
// use crate::packet::Packet;
use crate::types_packet::Packet;
use crate::types_header::HeaderAck;
use logger::{debug_println, log::{self, SharedLogger, MessageStatus}};

use std::clone::Clone;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Sender, Receiver, RecvTimeoutError};
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
    DEAD,
    // SUSPECT,
}

#[derive(PartialEq)]
pub enum Broadcast {
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
    RequestLeader {},
    // Creates N messages to gossip to neighbors, keeping the original message information
    Gossip {
        origin: SocketAddr,
        seq_num: u32,
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

/// Reads the arguments from the command line, parses them and returns them as a tuple
fn get_args() -> (Broadcast, Duration, u32, Duration, Duration, usize, usize) {
    let args: Vec<String> = std::env::args().collect();
    let broadcast: Broadcast = match args[1].as_str() {
        "BEB" => Broadcast::BEB,
        "URB" => Broadcast::URB,
        "AB" => Broadcast::AB,
        _ => panic!("Falha ao converter broadcast {} para Broadcast", args[3]),
    };
    let timeout: u64 = args[2]
        .parse()
        .expect("Falha ao converter timeout para u64");
    let timeout_limit: u32 = args[3]
        .parse()
        .expect("Falha ao converter timeout_limit para u32");
    let message_timeout: u64 = args[4]
        .parse()
        .expect("Falha ao converter message_timeout para u64");
    let broadcast_timeout: u64 = args[5]
        .parse()
        .expect("Falha ao converter broadcast_timeout para u64");
    let gossip_rate: usize = args[6]
        .parse()
        .expect("Falha ao converter gossip_rate para usize");
    let w_size: usize = args[7]
        .parse()
        .expect("Falha ao converter w_size para usize");

    let timeout = Duration::from_micros(timeout);
    let message_timeout = Duration::from_millis(message_timeout);
    let broadcast_timeout = Duration::from_millis(broadcast_timeout);
    (
        broadcast,
        timeout,
        timeout_limit,
        message_timeout,
        broadcast_timeout,
        gossip_rate,
        w_size,
    )
}

pub struct ReliableCommunication {
    pub host: Node,
    pub group: Mutex<Vec<Node>>,
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
    pub logger: SharedLogger,
}

impl ReliableCommunication {
    /// Starts a new thread to listen for any incoming messages
    /// This thread will be responsible for handling the destination of each received packet
    pub fn new(
        host: Node,
        group: Vec<Node>,
        logger: SharedLogger,
    ) -> Result<Arc<Self>, std::io::Error> {
        let channel = Channel::new(host.addr)?;
        let (broadcast, timeout, timeout_limit, message_timeout, broadcast_timeout, gossip_rate, w_size) = get_args();
        let group = Mutex::new(group);

        let (register_to_sender_tx, register_to_sender_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();

        let (register_to_listener_tx, register_to_listener_rx) = mpsc::channel();
        let (broadcast_waiters_tx, broadcast_waiters_rx) = mpsc::channel();
        let receive_rx = Mutex::new(receive_rx);

        let instance = Arc::new(Self {
            host,
            group,
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

        Ok(instance)
    }

    /// Send a message to a specific destination
    pub fn send(&self, id: usize, message: Vec<u8>) -> u32 {
        let node = {
            let g = self.group.lock().expect("Erro ao enviar mensagem: Mutex lock do grupo falhou");
            g.get(id).cloned()
        };
        match node {
            Some(node) => {
                match self.send_nonblocking(&node.addr, message).recv() {
                    Ok(result) => result,
                    Err(e) => {
                        debug_println!("Erro ao enviar mensagem na send: {e}");
                        0
                    }
                }
            },
            None => {
                debug_println!("Erro ao enviar mensagem: ID de destino não encontrado");
                0
            }
        }
    }

    fn send_nonblocking(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> Receiver<u32> {
        let (request, result_rx) = SendRequest::new(
            message,
            SendRequestData::Send {
                destination_address: *dst_addr,
            },
        );
        match self.register_to_sender_tx.send(request) {
            Ok(_) => {}
            Err(e) => {
                debug_println!("Erro ao registrar request: {e}");
            }
        }
        result_rx
    }

    /// Read one already received message or wait for a message to arrive
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        match self
            .receive_rx
            .lock()
            .expect("Erro ao receber mensagem: Mutex lock o receive_rx falhou")
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
            Broadcast::BEB => self.beb(message),
            Broadcast::URB => self.urb(message),
            Broadcast::AB => self.ab(message),
        }
    }

    /// Best-Effort Broadcast: attempts to send a message to all nodes in the group and return how many were successful
    /// This algorithm does not garantee delivery to all nodes if the sender fails
    fn beb(&self, message: Vec<u8>) -> u32 {
        let (request, result_rx) = SendRequest::new(message, SendRequestData::StartBroadcast {});
        match self.register_to_sender_tx.send(request) {
            Ok(_) => {}
            Err(e) => {
                debug_println!("Erro ao enviar request do BEB: {e}");
            }
        }

        match result_rx.recv() {
            Ok(result) => result,
            Err(e) => {
                debug_println!("Erro ao receber resultado do BEB: {e}");
                0
            }
        }
    }

    /// Uniform Reliable Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all nodes receive the message if the sender does not fail
    fn urb(&self, message: Vec<u8>) -> u32 {
        let (request, _) = SendRequest::new(message, SendRequestData::StartBroadcast {});
        match self.register_to_sender_tx.send(request) {
            Ok(_) => {}
            Err(e) => {
                debug_println!("Erro ao fazer URB: {e}");
            }
        }
        // TODO: Make it so that URB waits for at least one node to receive the message
        self.group.lock()
        .expect("Falha ao fazer o URB, Não obteve lock de Grupo").len() as u32
    }

    /// Atomic Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all messages are delivered in the same order to all nodes
    fn ab(&self, message: Vec<u8>) -> u32 {
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<Vec<u8>>();
        match self.broadcast_waiters_tx.send(broadcast_tx) {
            Ok(_) => {}
            Err(e) => {
                debug_println!("Erro ao registrar broadcast waiter no AB: {e}");
            }
        }
        
        // Constantly try to get a leader and ask it to broadcast
        loop {
            let leader = self.get_leader();
            if leader == self.host {
                // Start the broadcast
                let (request, _) = SendRequest::new(
                    message.clone(),
                    SendRequestData::StartBroadcast {},
                );
                self.register_to_sender_tx.send(request).expect("Erro ao fazer AB, não conseguiu enviar request para o sender");
                return self.group.lock().expect("Erro ao terminar o AB, não obteve-se o Mutex lock do grupo").len() as u32;
            }
            // Ask the leader to broadcast and wait for confirmation
            let (request, request_result_rx) = SendRequest::new (
                message.clone(),
                SendRequestData::RequestLeader {},
            );
            match self.register_to_sender_tx.send(request) {
                Ok(_) => {}
                Err(e) => {
                    debug_println!("Erro ao enviar request de AB: {e}");
                }
            }

            // Wait for the request result
            match request_result_rx.recv() {
                Ok(0) => {
                    debug_println!("Agente {} falhou em enviar mensagem para o líder, tentando novamente...", self.host.agent_number);
                    continue;
                }
                Ok(_) => {}
                Err(e) => {
                    debug_println!("Erro ao fazer requisitar AB para o lider: {e}");
                    continue;
                }
            }

            // Listen for any broadcasts until your message arrives
            // While there are broadcasts arriving, it means the leader is still alive
            // If the channel times out before your message arrives, it means the leader died
            loop {
                let msg = broadcast_rx.recv_timeout(self.broadcast_timeout);
                match msg {
                    Ok(msg) => {
                        if msg == message {
                            debug_println!("Agente {} recebeu a mensagem de broadcast de volta", self.host.agent_number);
                            return self.group.lock().expect("Erro ao terminar o AB, não obteve-se o Mutex lock do grupo").len() as u32;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        debug_println!("Agent {} timed out when waiting broadcast from leader, will try again", self.host.agent_number);
                        break;
                    }
                    Err(e) => {
                        panic!("{:?}", e)
                    }
                }
            }
        }
    }

    /// Marks a node as dead
    fn mark_as_dead(&self, addr: &SocketAddr) {
        let mut group = self.group.lock().expect("Erro ao marcar como morto: Mutex lock do grupo falhou");
        for node in group.iter_mut() {
            if node.addr == *addr {
                debug_println!("Agente {} marcou {} como morto", self.host.agent_number, node.agent_number);
                node.state = NodeState::DEAD;
            }
        }
    }

    /// Returns the node with the highest priority (currently the first one alive in the group vector)
    fn get_leader(&self) -> Node {
        for node in self.group.lock().expect("Falha ao ler do grupo").iter() {
            if node.state == NodeState::ALIVE {
                debug_println!("Agente {} escolheu {} como líder", self.host.agent_number, node.agent_number);
                return node.clone();
            }
        }
        return self.host.clone();
    }

    /// Calculates the priority of a node in the group (currently the lowest index in the group vector)
    fn get_leader_priority(&self, node_address: &SocketAddr) -> usize {
        let group = self.group.lock().expect("Erro ao obter prioridade do líder: Mutex lock do grupo falhou");
        for (i, n) in group.iter().enumerate() {
            if n.addr == *node_address {
                return group.len()-i;
            }
        }
        0
    }

    /// Picks the node "friends" and retransmits the message to them
    /// This retransmission preserves the original message information about the origin and sequence number
    /// The friends are any group of N nodes in the group, where N is the gossip rate. Currently it's the next N nodes in the group vector
    /// 
    /// Since gossip algorithms are meant to ensure that the message will be successfully difused even if there are failing nodes
    /// This function doesn't need to wait for the result of the gossip. (It's also important to not block the listener thread when it needs to gossip a message)
    fn gossip(&self, data: Vec<u8>, origin: SocketAddr, seq_num: u32) {
        let (request, _) = SendRequest::new(
            data,
            SendRequestData::Gossip {
                origin,
                seq_num,
            },
        );
        match self.register_to_sender_tx.send(request) {
            Ok(_) => {}
            Err(e) => {
                debug_println!("Erro ao fazer fofoca: {e}");
            }
        }
    }

    /// Thread to handle the sending of messages
    fn sender(
        self: Arc<Self>,
        acks_rx: Receiver<HeaderAck>,
        register_from_user_rx: Receiver<SendRequest>,
        register_to_listener_tx: Sender<((SocketAddr, SocketAddr), u32)>,
    ) {
        // TODO: Upgrade this thread to make it able of sending multiple messages at once
        while let Ok(request) = register_from_user_rx.recv() {
            let messages_to_send = self.get_messages(&request);
            let mut success_count = 0;
            for packets in messages_to_send {
                // Register the destination address and the sequence to the listener thread
                let first = match packets.first() {
                    Some(packet) => packet,
                    None => {
                        debug_println!("Erro ao enviar mensagem: Pacote vazio");
                        continue;
                    }
                };
                match register_to_listener_tx.send(
                    ((first.get_dst_addr(),
                    first.get_origin_addr()),
                    first.get_seq_num(),)
                ) {
                    Ok(_) => {
                        match first {
                            Packet::Send {header, ..} => {
                                let logger_state = log::LoggerState::Message{
                                    state: MessageStatus::SentBroadcast,
                                    current_agent_id: Some(self.host.agent_number),
                                    target_agent_id: Some(header.dst_addr.port() as usize % 100),
                                    message_id: header.seq_num as usize,
                                };
                                self.logger
                                    .lock()
                                    .expect("Couldn't acquire logger Lock on Sender")
                                    .log(logger_state);
                            } 
                            Packet::Broadcast { header, .. } => {
                                let logger_state = log::LoggerState::Message{
                                    state: MessageStatus::Sent,
                                    current_agent_id: Some(self.host.agent_number),
                                    target_agent_id: Some(header.dst_addr.port() as usize % 100),
                                    message_id: header.seq_num as usize,
                                };

                                self.logger
                                    .lock()
                                    .expect("Couldn't acquire logger Lock on Sender")
                                    .log(logger_state);
                            }
                            Packet::Ack { .. } => panic!("Erro ao enviar mensagem: Acks não devem ser enviados por Sender"),
                        }
                    }
                    Err(e) => {
                        self.logger
                            .lock()
                            .expect("Couldn't aquire logger Lock on Sender")
                            .fail(
                                format!("Erro ao enviar pedido de ACK para a Listener: {e}")
                                    .to_owned(),
                                Some(self.host.agent_number),
                            );
                        debug_println!("Erro ao enviar pedido de ACK para a Listener: {e}");
                        continue;
                    }
                }

                // Go back-N algorithm to send packets
                if self.go_back_n(&packets, &acks_rx) {
                    success_count += 1;
                } else {
                    // If the message wasn't sent, mark the destination as dead
                    self.mark_as_dead(&first.get_dst_addr());
                }
            }

            // Notify the caller that the request was completed
            // The caller may have chosen to not wait for the result, so we ignore if the channel was disconnected
            let _ = request.result_tx.send(success_count);
        }
    }

    /// Go-Back-N algorithm to send packets
    fn go_back_n(&self, packets: &Vec<Packet>, acks_rx: &Receiver<HeaderAck>) -> bool {
        let mut base = 0;
        let mut next_seq_num = 0;
        let mut timeout_count = 0;
        let start_packet = match packets.first() {
            Some(packet) => {
                match packet {
                    Packet::Send {header, ..} => header.seq_num,
                    Packet::Broadcast {header, ..} => header.seq_num,
                    Packet::Ack { .. } => panic!("Erro ao enviar mensagem: Pacote de ACK não deve ser enviado por Go-Back-N"),
                }},
            None => {
                debug_println!("Erro ao enviar mensagem: Pacote vazio");
                return false;
            }
        };
        while base < packets.len() {
            // Send window
            while next_seq_num < base + self.w_size && next_seq_num < packets.len() {
                self.channel.send(&packets[next_seq_num]);
                
                // logger
                {
                    match &packets[0] {
                        Packet::Broadcast {header, ..} => {
                            let logger_state = log::LoggerState::Packet{
                                state: log::PacketStatus::SentBroadcast,
                                current_agent_id: Some(self.host.addr.port() as usize % 100),
                                target_agent_id: Some(header.dst_addr.port() as usize % 100),
                                seq_num: next_seq_num,
                            };
                            self.logger
                                .lock()
                                .expect("Couldn't aquire logger Lock on Go-Back-N")
                                .log(logger_state);

                        }
                        Packet::Send {header, .. } => {
                            let logger_state = log::LoggerState::Packet{
                                state: log::PacketStatus::Sent,
                                current_agent_id: Some(self.host.addr.port() as usize % 100),
                                target_agent_id: Some(header.dst_addr.port() as usize % 100),
                                seq_num: next_seq_num,
                            };
                            self.logger
                                .lock()
                                .expect("Couldn't aquire logger Lock on Go-Back-N")
                                .log(logger_state);
                        }
                        Packet::Ack { .. } => panic!("Erro ao enviar mensagem: Pacote de ACK não deve ser enviado por Go-Back-N"),
                    }
                }
                next_seq_num += 1;
            }


            // Wait for an ACK
            // TODO: Somewhere around here: add logic to check if destination is still alive, if not, break the loop, reset the sequence number and return false
            match acks_rx.recv_timeout(self.timeout) {
                Ok(ack) => {
                    // Assume that the listener is sending the number of the highest packet it received
                    // The listener also guarantees that the packet is >= base
                    base = (ack.seq_num - start_packet as u32 + 1) as usize;                    
                }
                Err(RecvTimeoutError::Timeout) => {
                    next_seq_num = base;
                    timeout_count += 1;
                    if timeout_count == self.timeout_limit {
                        debug_println!("Agent {} timed out when sending message to agent {}", self.host.agent_number, packets.first().expect("Packets vazio no Go-Back-N").get_dst_addr().port()%100);
                        return false;
                    }
                },
                Err(RecvTimeoutError::Disconnected) => {
                    debug_println!("Erro ao receber ACKS, thread Listener desconectada");
                    return false;
                },
            }
        }
        true
    }

    /// Handles a request for the sender, returning what messages the request generates
    /// For example: A simple send request will generate one message, while a broadcast request will generate N messages
    fn get_messages(&self, request: &SendRequest) -> Vec<Vec<Packet>> {
        let mut messages = Vec::new();
        match &request.options {
            SendRequestData::Send { destination_address } => {
                let packets = self.get_pkts( &self.host.addr,destination_address, &self.host.addr, request.data.clone(), false);
                messages.push(packets);
            }
            SendRequestData::StartBroadcast {} => match self.broadcast {
                Broadcast::BEB => {
                    for node in self.group.lock().expect("Couldn't get grupo lock on get_messages").iter() {
                        if node.state == NodeState::ALIVE {
                            let packets = self.get_pkts(&self.host.addr, &node.addr, &self.host.addr, request.data.clone(), false);
                            messages.push(packets);
                        }
                    }
                }
                Broadcast::URB | Broadcast::AB => {
                    let friends = self.get_friends();
                    for node in self.group.lock().expect("Couldn't get grupo lock on get_messages").iter() {
                            let packets = self.get_pkts(&self.host.addr, &node.addr, &self.host.addr, request.data.clone(), true);
                            if friends.contains(node) && node.state == NodeState::ALIVE {
                                messages.push(packets);
                            }
                        }
                    }
            },
            SendRequestData::RequestLeader {} => {
                let leader = self.get_leader();
                let packets = self.get_pkts(&self.host.addr, &leader.addr, &self.host.addr, request.data.clone(), true);
                messages.push(packets);
            }
            SendRequestData::Gossip { origin, seq_num } => {
                for node in self.get_friends() {
                    let packets = Packet::brdcst_pkts_from_message(
                        self.host.addr,
                        node.addr,
                        *origin,
                        *seq_num,
                        request.data.clone(),
                    );
                    if node.state == NodeState::ALIVE {
                        messages.push(packets);
                    }
                }
            }
        }  
        messages
    }

    /// Returns the nodes that are considered friends of the current node
    /// Currently, the friends are the next N nodes in the group vector, where N is the gossip rate
    fn get_friends(&self) -> Vec<Node> {
        let group = self.group.lock().expect("Couldn't get grupo lock on get_friends");

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

    /// Builds the packets based on the message and the destination. Will also update the sequence number counter for the destination
    fn get_pkts(
        &self,
        src_addr: &SocketAddr,
        dst_addr: &SocketAddr,
        origin: &SocketAddr,
        data: Vec<u8>,
        is_brd: bool
    ) -> Vec<Packet> {
        let mut seq_lock = self.dst_seq_num_cnt.lock().expect("Erro ao obter lock de dst_seq_num_cnt em get_pkts");
        let start_seq = seq_lock.entry(*dst_addr).or_insert(0);
        let packets = if is_brd {
            Packet::brdcst_pkts_from_message(
            *src_addr, *dst_addr, *origin, *start_seq, data,
            )
        } else {
            Packet::send_pkts_from_message(
                *src_addr, *dst_addr, *start_seq, data,
            )
        };
        *start_seq += packets.len() as u32;
        packets
    }

    /// Thread to handle the reception of messages
    fn listener(
        self: Arc<Self>,
        messages_tx: Sender<Vec<u8>>,
        acks_tx: Sender<HeaderAck>,
        register_from_sender_rx: Receiver<((SocketAddr, SocketAddr), u32)>,
        register_broadcast_waiters_rx: Receiver<Sender<Vec<u8>>>,
    ) {
        let mut pkts_per_origin: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut expected_acks: HashMap<(SocketAddr, SocketAddr), u32> = HashMap::new();
        let mut broadcast_waiters: Vec<Option<Sender<Vec<u8>>>> = Vec::new();
        loop {
            let packet = match self.channel.receive() {
                Ok(packet) => packet,
                Err(e) => {
                    debug_println!("Agente {} falhou em receber um pacote do socket, erro: {e}", self.host.agent_number);
                    continue;
                }
            };
            match &packet {
                Packet::Ack { header, .. } => {

                    {
                            let logger_state = log::LoggerState::Packet{
                            state: log::PacketStatus::ReceivedAck,
                            current_agent_id: Some(header.dst_addr.port() as usize % 100),
                            target_agent_id: Some(header.src_addr.port() as usize % 100),
                            seq_num: header.seq_num as usize,
                        };

                        self.logger
                            .lock()
                            .expect("Couldn't acquire logger Lock on Listener")
                            .log(logger_state);
                    }

                    // Handle ack
                    while let Ok((key, start_seq)) = register_from_sender_rx.try_recv() {
                        expected_acks.insert(key, start_seq);
                    }
                    match expected_acks.get_mut(&(header.src_addr, header.src_addr)) {
                        Some(seq_num) => {
                            if header.seq_num < *seq_num { continue; }
                            *seq_num = header.seq_num + 1;
                            match acks_tx.send(header.clone()) {
                                Ok(_) => {}
                                Err(e) => {
                                    debug_println!("Erro ao enviar ACK: {e}");
                                        {
                                            let logger_state = log::LoggerState::Packet{
                                                state: log::PacketStatus::ReceivedAckFailed,
                                                current_agent_id: Some(header.dst_addr.port() as usize % 100),
                                                target_agent_id: Some(header.src_addr.port() as usize % 100),
                                                seq_num: header.seq_num as usize,
                                            };
                                            self.logger
                                            .lock()
                                            .expect("Couldn't acquire logger Lock on Listener")
                                            .log(logger_state);
                                    }
                                }
                            }
                        }
                        None => {
                            debug_println!("ACK recebido sem destinatário esperando");
                            let logger_state = log::LoggerState::Packet{
                                state: log::PacketStatus::ReceivedAckFailed,
                                current_agent_id: Some(header.dst_addr.port() as usize % 100),
                                target_agent_id: Some(header.src_addr.port() as usize % 100),
                                seq_num: header.seq_num as usize,
                            };

                            self.logger
                            .lock()
                            .expect("Couldn't acquire logger Lock on Listener")
                            .log(logger_state);    
                        }
                    }
                } 
                Packet::Broadcast { header, .. } => {
                    {
                        let logger_state = log::LoggerState::Packet{
                            state: log::PacketStatus::Received,
                            current_agent_id: Some(header.dst_addr.port() as usize % 100),
                            target_agent_id: Some(header.src_addr.port() as usize % 100),
                            seq_num: header.seq_num as usize,

                        };

                        self.logger.lock().expect("Couldn't acquire logger Lock on Listener").log(logger_state);
                    }

                    // Handle data
                    let packets = pkts_per_origin
                        .entry(header.origin)
                        .or_insert(Vec::new());
                    let expected = match packets.last() {
                        Some(p) => match p {
                            Packet::Broadcast { header, .. } => header.seq_num + 1,
                            _ => panic!("Erro ao receber mensagem: Último pacote não é um Broadcast"),
                        },
                        None => 0,
                    };

                    // Ignore the packet if the sequence number is higher than expected
                    if header.seq_num > expected {
                        continue;
                    }
                    // Send ack otherwise
                    self.channel.send(&packet.get_ack());
                    {
                        let logger_state = log::LoggerState::Packet {
                            state: log::PacketStatus::SentAck,
                            current_agent_id: Some(header.dst_addr.port() as usize % 100),
                            target_agent_id: Some(header.src_addr.port() as usize % 100),
                            seq_num: header.seq_num as usize,
                        };
                        self.logger.lock().expect("Couldn't acquire logger Lock on Listener").log(logger_state);
                    }

                    if header.seq_num < expected { continue; }
                    
                    if header.is_last {
                        let (message, origin, sequence_number) =
                            ReliableCommunication::receive_last_packet(&self, packets, &packet);
                        match self.broadcast {
                            // BEB: All broadcasts must be delivered
                            Broadcast::BEB => {
                                debug_println!("Erro: Pedido de fofoca em um sistema sem fofoca");
                                messages_tx.send(message).expect("Erro ao enviar mensagem para a aplicação");
                            }
                            // URB: All broadcasts must be gossiped and delivered
                            Broadcast::URB => {
                                self.gossip(message.clone(), origin, sequence_number);
                                messages_tx.send(message).expect("Erro ao enviar mensagem para a aplicação");
                            }
                            // AB: Must check if I'm the leader and should broadcast it or just gossip
                            // In AB, broadcast messages can only be delivered if they were sent by the leader
                            // However, since the leader is the only one that can broadcast,
                            // this is only useful for the leader,
                            // who must not deliver the request to broadcast to the group,
                            // instead, it must broadcast the message and only deliver when it gets gossiped back to it
                            Broadcast::AB => {
                                // Check for anyone waiting for a broadcast
                                while let Ok(broadcast_waiter) = register_broadcast_waiters_rx.try_recv() {
                                    broadcast_waiters.push(Some(broadcast_waiter));
                                }
                                for waiter in broadcast_waiters.iter_mut() {
                                    let w = waiter.as_ref().unwrap();
                                    match (*w).send(message.clone()) {
                                        Ok(_) => {} 
                                        Err(_) => {
                                            *waiter = None;
                                        }
                                    }
                                }
                                broadcast_waiters.retain(|w| w.is_some());
                                
                                if self.atm_gossip(message.clone(), &origin, sequence_number) {
                                    messages_tx.send(message).expect("Erro ao enviar mensagem para a aplicação");
                                }
                            }
                        }
                    }
                    packets.push(packet);
                }
                Packet::Send { header, .. } => {
                    {
                        let logger_state = log::LoggerState::Packet{
                            state: log::PacketStatus::Received,
                            current_agent_id: Some(header.dst_addr.port() as usize % 100),
                            target_agent_id: Some(header.src_addr.port() as usize % 100),
                            seq_num: header.seq_num as usize,

                        };

                        self.logger.lock().expect("Couldn't acquire logger Lock on Listener").log(logger_state);
                    }

                    // Handle data
                    let packets = pkts_per_origin
                        .entry(header.src_addr)
                        .or_insert(Vec::new());
                    let expected = match packets.last() {
                        Some(p) => match p {
                            Packet::Send { header, .. } => header.seq_num + 1,
                            _ => panic!("Erro ao receber mensagem: Último pacote não é um Send"),
                        },
                        None => 0,
                    };

                    // Ignore the packet if the sequence number is higher than expected
                    if header.seq_num > expected {
                        continue;
                    }
                    // Send ack otherwise
                    self.channel.send(&packet.get_ack());
                    {
                        let logger_state = log::LoggerState::Packet {
                            state: log::PacketStatus::SentAck,
                            current_agent_id: Some(header.dst_addr.port() as usize % 100),
                            target_agent_id: Some(header.src_addr.port() as usize % 100),
                            seq_num: header.seq_num as usize,
                        };
                        self.logger.lock().expect("Couldn't acquire logger Lock on Listener").log(logger_state);
                    }

                    if header.seq_num < expected { continue; }
                    
                    if header.is_last {
                        let (message, _, _) =
                            ReliableCommunication::receive_last_packet(&self, packets, &packet);
                        messages_tx.send(message).expect("Erro ao enviar mensagem para a aplicação");
                    }
                    packets.push(packet);
                }
        }
    }
    }

    /// Decides what to do with a broadcast message in the AB algorithm
    /// Based on your priority and the priority of the origin of the message
    /// The return boolean is used to tell the listener thread whether the message should be delivered or not (in case it's a broadcast request for the leader)
    fn atm_gossip(
        &self,
        message: Vec<u8>,
        origin: &SocketAddr,
        sequence_number: u32
    ) -> bool {
        let origin_priority = self.get_leader_priority(&origin);
        let own_priority = self.get_leader_priority(&self.host.addr);
        if origin_priority < own_priority {
            // If the origin priority is lower than yours, it means the the origin considers you the leader and you must broadcast the message
            self.ab(message);
            false
        } else {
            // If the origin priority is higher or equal to yours, it means the origin is the leader and you must simply gossip the message
            self.gossip(message.clone(), *origin, sequence_number);
            true
        }
    }

    /// When a packet marked as last is received, the packets are merged and the message is returned
    fn receive_last_packet(
        &self,
        packets: &mut Vec<Packet>,
        packet: &Packet
    ) -> (Vec<u8>, SocketAddr, u32) {
        let mut message = Vec::new();
        // Ignore the first packet if its the remnant of a previous message
        match packets.first() { 
            Some(p) => {
                if p.is_last() { packets.remove(0); }
            }
            None => {}
         }

        for packet in packets.iter() {            
            message.extend(&packet.get_data());
        }
        message.extend(&packet.get_data());

        let sequence_number: u32;
        {
            let curr_agent_id: usize;
            let t_agent_id: usize;
            match packets.first() {
                Some(p) => {
                    sequence_number = p.get_seq_num();
                    curr_agent_id = p.get_src_addr().port() as usize % 100;
                    t_agent_id = p.get_dst_addr().port() as usize % 100;
            }
                None => {
                    sequence_number = packet.get_seq_num();
                    curr_agent_id = packet.get_src_addr().port() as usize % 100;
                    t_agent_id = packet.get_dst_addr().port() as usize % 100;
                }
            }

            let logger_state = log::LoggerState::Packet {
                state: log::PacketStatus::ReceivedLastPacket,
                current_agent_id: Some(curr_agent_id),
                target_agent_id: Some(t_agent_id),
                seq_num: sequence_number as usize,
            };

            self.logger
                .lock()
                .expect("Couldn't acquire logger Lock on Sender")
                .log(logger_state);
        }

        packets.clear();
        (message, packet.get_origin_addr(), sequence_number)
    }
}
