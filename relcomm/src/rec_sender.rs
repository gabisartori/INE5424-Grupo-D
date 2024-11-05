use std::net::SocketAddr;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender, RecvTimeoutError};

use crate::channels::Channel;
use crate::packet::Packet;
use crate::node::{Node, NodeState, Broadcast};
use crate::rec_aux::{SendRequest, SendRequestData, RecAux};
use logger::debug_println;
use logger::log::{MessageStatus, PacketStatus, SharedLogger};

// TODO: reduce the need for self parameters by separating elements that don't need to be shared 
/// Sender thread that handles the sending of messages
pub struct RecSender {
    host: Node,
    dst_seq_num_cnt: Mutex<HashMap<SocketAddr, u32>>,
    channel: Arc<Channel>,
    // TODO: Make save the sequence number counter in the group
    group: Arc<Mutex<Vec<Node>>>,
    broadcast: Broadcast,
    logger: SharedLogger,
    timeout: Duration,
    timeout_limit: u32,
    w_size: usize,
    gossip_rate: usize,
}

impl RecSender {
    /// Constructor
    pub fn new(
        host: Node,
        dst_seq_num_cnt: Mutex<HashMap<SocketAddr, u32>>,
        channel: Arc<Channel>,
        group: Arc<Mutex<Vec<Node>>>,
        broadcast: Broadcast,
        logger: SharedLogger,
        timeout: Duration,
        timeout_limit: u32,
        w_size: usize,
        gossip_rate: usize,
    ) -> Self {
        Self {
            host,
            dst_seq_num_cnt,
            channel,
            group,
            broadcast,
            logger,
            timeout,
            timeout_limit,
            w_size,
            gossip_rate,
        }
    }

    /// Thread to handle the sending of messages
    pub fn run(&self,
        acks_rx: Receiver<Packet>,
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
                    Some(packet) => packet.clone(),
                    None => {
                        debug_println!("Erro ao enviar mensagem: Pacote vazio");
                        continue;
                    }
                };
                match register_to_listener_tx.send((
                    (first.header.dst_addr, first.header.origin),
                    first.header.seq_num,
                )) {
                    Ok(_) => {
                        let state = if first.header.must_gossip() {
                            MessageStatus::SentBroadcast
                        } else {
                            MessageStatus::Sent
                        };
                        RecAux::log_msg(&self.logger, &self.host, &first, state);
                    }
                    Err(e) => {
                        self.logger
                            .lock()
                            .expect("Couldn't aquire logger Lock on Sender")
                            .fail(
                                format!("Erro ao enviar pedido de ACK para a Listener: {e}")
                                    .to_owned(),
                                Some(packets[0].header.src_addr.port() as usize % 100),
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
                    self.mark_as_dead(&first.header.dst_addr);
                }
            }

            // Notify the caller that the request was completed
            // The caller may have chosen to not wait for the result, so we ignore if the channel was disconnected
            let _ = request.result_tx.send(success_count);
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
    /// Go-Back-N algorithm to send packets
    fn go_back_n(&self, packets: &Vec<Packet>, acks_rx: &Receiver<Packet>) -> bool {
        let mut base = 0;
        let mut next_seq_num = 0;
        let mut timeout_count = 0;
        let start_packet = match packets.first() {
            Some(packet) => packet.header.seq_num,
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
                let state = if packets[next_seq_num].header.must_gossip() {
                    PacketStatus::SentBroadcast
                } else {
                    PacketStatus::Sent
                };
                RecAux::log_pkt(&self.logger, &self.host, &packets[next_seq_num], state);

                next_seq_num += 1;
            }


            // Wait for an ACK
            // TODO: Somewhere around here: add logic to check if destination is still alive, if not, break the loop, reset the sequence number and return false
            match acks_rx.recv_timeout(self.timeout) {
                Ok(packet) => {
                    // Assume that the listener is sending the number of the highest packet it received
                    // The listener also guarantees that the packet is >= base
                    base = (packet.header.seq_num - start_packet as u32 + 1) as usize;                    
                }
                Err(RecvTimeoutError::Timeout) => {
                    next_seq_num = base;
                    timeout_count += 1;
                    if timeout_count == self.timeout_limit {
                        debug_println!("Agent {} timed out when sending message to agent {}", self.host.agent_number, packets.first().expect("Packets vazio no Go-Back-N").header.dst_addr.port()%100);
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

    /// Currently, the friends are the next N nodes in the group vector, where N is the gossip rate
    fn get_friends(&self) -> Vec<Node> {
        let mut old_group = self.group.lock().expect("Couldn't get grupo lock on get_friends");
        let group = old_group.iter_mut().filter(|n| n.state == NodeState::ALIVE).map(|n| n.clone()).collect::<Vec<Node>>();

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
    }/// Builds the packets based on the message and the destination. Will also update the sequence number counter for the destination
    fn get_pkts(
        &self,
        src_addr: &SocketAddr,
        dst_addr: &SocketAddr,
        origin: &SocketAddr,
        data: Vec<u8>,
        is_gossip: bool
    ) -> Vec<Packet> {
        let mut seq_lock = self.dst_seq_num_cnt.lock().expect("Erro ao obter lock de dst_seq_num_cnt em get_pkts");
        let start_seq = seq_lock.entry(*dst_addr).or_insert(0);
        let packets = Packet::packets_from_message(
            *src_addr, *dst_addr, *origin, data, *start_seq, is_gossip,
        );
        *start_seq += packets.len() as u32;
        packets
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
                            if friends.contains(node) {
                                messages.push(packets);
                            }
                        }
                    }
            },
            SendRequestData::RequestLeader {} => {
                let leader = RecAux::get_leader(&self.group, &self.host);
                let packets = self.get_pkts(&self.host.addr, &leader.addr, &self.host.addr, request.data.clone(), true);
                messages.push(packets);
            }
            SendRequestData::Gossip { origin, seq_num } => {
                for node in self.get_friends() {
                    let packets = Packet::packets_from_message(
                        self.host.addr,
                        node.addr,
                        *origin,
                        request.data.clone(),
                        *seq_num,
                        true,
                    );
                    messages.push(packets);
                }
            }
        }  
        messages
    }

}
