use std::time::Duration;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, RecvTimeoutError};

use logger::{log::{PacketStatus, MessageStatus, SharedLogger}, debug};
use crate::rec_aux::{SendRequest, Broadcast, SendRequestData, RecAux};
use crate::config::{GOSSIP_RATE, HEARTBEAT_MISS_LIMIT, TIMEOUT, TIMEOUT_LIMIT, W_SIZE};
use crate::node::Node;
use crate::channels::Channel;
use crate::packet::Packet;

/// Sender thread that handles the sending of messages
pub struct RecSender {
    host: Node,
    group: Arc<Mutex<Vec<Node>>>,
    channel: Arc<Channel>,
    logger: SharedLogger,
    // Keeps track of the sequence number, for sends and broadcasts
    dst_seq_num_cnt: Mutex<HashMap<SocketAddr, (u32, u32)>>,
    broadcast: Broadcast,
    timeout: Duration,
    timeout_limit: u32,
    w_size: usize,
    gossip_rate: usize,
    reg_to_snd_tx: Sender<SendRequest>,
}

impl RecAux for RecSender {}

impl RecSender {
    /// Constructor
    pub fn new( host: Node, group: Arc<Mutex<Vec<Node>>>,
        channel: Arc<Channel>, reg_to_snd_tx: Sender<SendRequest>, broadcast: Broadcast,
        logger: SharedLogger) -> Self {
        Self {
            host,
            group,
            channel,
            logger,
            dst_seq_num_cnt: Mutex::new(HashMap::new()),
            broadcast,
            timeout: TIMEOUT,
            timeout_limit: TIMEOUT_LIMIT,
            w_size: W_SIZE,
            gossip_rate: GOSSIP_RATE,
            reg_to_snd_tx,
        }
    }

    /// Thread to handle the sending of messages
    pub fn run(&self,
        snd_acks_rx: Receiver<Packet>,
        brd_acks_rx: Receiver<Packet>,
        reg_to_send_rx: Receiver<SendRequest>,
        reg_snd_to_listener_tx: Sender<((SocketAddr, SocketAddr), u32)>,
        reg_brd_to_listener_tx: Sender<((SocketAddr, SocketAddr), u32)>,
    ) {
        while let Ok(request) = reg_to_send_rx.recv() {
            let messages_to_send = self.get_messages(&request);
            if messages_to_send.is_empty() {
                // If there are no messages to send, we can skip the rest of the loop
                debug!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>\nNenhuma mensagem para enviar");
                let _ = request.result_tx.send(0);
                continue;
            }
            // logger
            let state = if messages_to_send[0][0].header.is_brd() {
                MessageStatus::SentBroadcast
            } else {
                MessageStatus::Sent
            };
            let mut success_count = 0;
            for packets in messages_to_send {
                // Register the destination address and the sequence to the listener thread
                let first = &packets[0];                
                let target = {
                    let group = self.group
                        .lock()
                        .expect("Erro ao obter lock de grupo em run");
                    group.iter()
                        .find(|node| node.addr == first.header.dst_addr)
                        .expect("Invalid Address").clone()
                };
                if target.is_dead() {
                    debug!("Erro ao enviar mensagem: {} está morto", target);
                    Self::log_msg(&self.logger, &self.host, &first, MessageStatus::SentFailed);
                    self.reset_seq_num(&first);
                    continue;
                }
                let (reg, acks_rx) = {
                    if first.header.is_brd() {
                        (&reg_brd_to_listener_tx, &brd_acks_rx)
                    } else {
                        (&reg_snd_to_listener_tx, &snd_acks_rx)
                    }
                };
                match reg.send((
                    (first.header.dst_addr, first.header.origin),
                    first.header.seq_num,
                )) {
                    Ok(_) => {
                        Self::log_msg(&self.logger, &self.host, &first, state);
                    }
                    Err(e) => {
                        debug!("Erro ao enviar pedido de ACK para a Listener: {e}");
                        Self::log_pkt(&self.logger, &self.host, &first, PacketStatus::SentAckFailed);
                        continue;
                    }
                }

                // Go back-N algorithm to send packets
                if self.go_back_n(&packets, &acks_rx) {
                    success_count += 1;
                }
                else {
                    self.resend_request(packets, &request);
                }
            }
            // Notify the caller that the request was completed
            let _ = request.result_tx.send(success_count);
            // The caller may have chosen to not wait for the result,
            // so we ignore if the channel was disconnected
        }
    }

    /// Resends a request, resetting the sequence number of all nodes
    fn resend_request(&self, pkts: Vec<Packet>, req: &SendRequest) {
        let attempts = if let SendRequestData::ResendPkt { attempts, .. } = req.options { attempts + 1} else { 1 };
        if attempts >= (HEARTBEAT_MISS_LIMIT/2) as u32 {
            let mut group = self.group
                .lock()
                .expect("Erro ao obter lock de grupo em resend_request");
            let target = group.iter_mut()
                .find(|node| node.addr == pkts[0].header.dst_addr)
                .expect("Invalid Address");
            target.set_as_dead();
            debug!("Erro ao re-enviar mensagem: Tentativas {attempts} esgotadas");
            self.reset_seq_num(&pkts[0]);
            return;
        }
        debug!("Reenviando mensagem com {} tentativas", attempts);
        let req = SendRequest::new(Vec::new(), SendRequestData::ResendPkt { pkts, attempts }).0;
        match self.reg_to_snd_tx.send(req) {
            Ok(_) => {}
            Err(e) => {
                debug!("Erro ao registrar request: {e}");
            }
        }
    }

    /// Handles a request for the sender, returning what messages the request generates
    /// For example: A simple send request will generate one message, while a broadcast request will generate N messages
    fn get_messages(&self, request: &SendRequest) -> Vec<Vec<Packet>> {
        let mut messages = Vec::new();
        match &request.options {
            SendRequestData::Send { dst_addr } => {
                let packets = self.get_pkts( &self.host.addr, dst_addr, &self.host.addr, request.data.clone(), false);
                debug!("Starting send from {}", packets[0]);
                messages.push(packets);
            },
            SendRequestData::RequestLeader {} => {
                let leader = Self::get_leader(&self.group, &self.host);
                let packets = self.get_pkts(&self.host.addr, &leader.addr, &self.host.addr, request.data.clone(), true);
                debug!("Requesting leader with {}", packets[0]);
                messages.push(packets);
            },
            SendRequestData::Gossip { origin, seq_num } => {
                debug!("Gossiping msg from Agent {}, with seq_num {}", Self::get_agnt(&origin), seq_num);
                for node in self.get_friends() {
                    let packets = Packet::packets_from_message(
                        self.host.addr,
                        node,
                        *origin,
                        request.data.clone(),
                        *seq_num,
                        true,
                    );
                    messages.push(packets);
                }
            },
            SendRequestData::StartBroadcast {} => {
                debug!("Starting broadcast");
                match self.broadcast {
                    Broadcast::BEB => {
                        for node in self.group
                            .lock()
                            .expect("Couldn't get grupo lock on get_messages")
                            .iter()
                        {
                            let packets = self.get_pkts(&self.host.addr, &node.addr, &self.host.addr, request.data.clone(), true);
                            messages.push(packets);
                        }
                    }
                    Broadcast::URB | Broadcast::AB => {
                        let friends = self.get_friends();
                        for node in self.group
                            .lock()
                            .expect("Couldn't get grupo lock on get_messages")
                            .iter()
                        {
                            let packets = self.get_pkts(&self.host.addr, &node.addr, &self.host.addr, request.data.clone(), true);
                            if friends.contains(&node.addr) {
                                messages.push(packets);
                            }
                        }
                    }
                }
            },
            SendRequestData::ResendPkt { pkts,.. } => {
                messages.push(pkts.clone());
            },
        }  
        messages
    }


    fn reset_seq_num(&self, first: &Packet) {
        let mut seq_lock = self.dst_seq_num_cnt
            .lock()
            .expect("Erro ao obter lock de dst_seq_num_cnt em reset_seq_num");
        let start_seq_tup = seq_lock.entry(first.header.dst_addr).or_insert((0, 0));
        if first.header.is_brd() {
            debug!("->-> Agent {} brd seq_num was {} and was reset to {}", Self::get_agnt(&first.header.dst_addr), start_seq_tup.1, first.header.seq_num);
            start_seq_tup.1 = first.header.seq_num;
        } else {
            debug!("->->-> Agent {} Send seq_num was {} and was reset to {}", Self::get_agnt(&first.header.dst_addr), start_seq_tup.1, first.header.seq_num);
            start_seq_tup.0 = first.header.seq_num;
        }
    }

    /// Builds the packets based on the message and the destination.
    /// Will also update the sequence number counter for the destination
    fn get_pkts(&self, src_addr: &SocketAddr, dst_addr: &SocketAddr,
        origin: &SocketAddr, data: Vec<u8>, is_brd: bool) -> Vec<Packet>
    {
        let mut seq_lock = self.dst_seq_num_cnt
            .lock()
            .expect("Erro ao obter lock de dst_seq_num_cnt em get_pkts");
        let start_seq_tup = seq_lock.entry(*dst_addr).or_insert((0, 0));
        let seq_num = if is_brd {&mut start_seq_tup.1} else {&mut start_seq_tup.0};
        let packets = Packet::packets_from_message(
            *src_addr, *dst_addr, *origin, data, *seq_num, is_brd,
        );
        debug!("<< Agent {} seq_num was {} and was set to {}", Self::get_agnt(dst_addr), *seq_num, *seq_num + packets.len() as u32);
        *seq_num += packets.len() as u32;
        packets
    }

    /// Go-Back-N algorithm to send packets
    fn go_back_n(&self, packets: &Vec<Packet>, acks_rx: &Receiver<Packet>) -> bool {
        let mut base = 0;
        let mut next_seq_num = 0;
        let mut timeout_count = 0;
        let first = match packets.first() {
            Some(packet) => packet.header.clone(),
            None => {
                debug!("Erro ao enviar mensagem: Pacote vazio");
                return false;
            }
        };
        // logger
        let state = {
            if first.is_brd() {
                PacketStatus::SentBroadcast
            } else {
                PacketStatus::Sent
            }
        };
        while base < packets.len() {
            // Send window
            while next_seq_num < base + self.w_size && next_seq_num < packets.len() {
                self.channel.send(&packets[next_seq_num]);
                next_seq_num += 1;
                // logger
                Self::log_pkt(&self.logger, &self.host, &packets[next_seq_num-1], state);
            }

            // Wait for an ACK
            match acks_rx.recv_timeout(self.timeout) {
                Ok(packet) => {
                    // Assume that the listener is sending the number of the highest packet it received
                    // The listener also guarantees that the packet is >= base
                    base = (packet.header.seq_num - first.seq_num as u32 + 1) as usize;
                    timeout_count = 0;
                },
                Err(RecvTimeoutError::Timeout) => {
                    next_seq_num = base;
                    timeout_count += 1;
                    {
                        let group = self.group
                            .lock()
                            .expect("Erro ao obter lock de grupo em go_back_n");
                        let target = group.iter()
                            .find(|node| node.addr == first.dst_addr)
                            .expect("Invalid Address");
                        if target.is_dead() {
                            debug!("Erro ao enviar mensagem: Agent {} não está vivo", Self::get_agnt(&first.dst_addr));
                            return false;
                        }
                        // if the target is suspect or not initiated,
                        // there will be a limit to the number of timeouts
                        if (!target.is_alive()) && timeout_count == self.timeout_limit {
                            debug!("Timed out {TIMEOUT_LIMIT
                                } times when waiting for ACK from Agent {}",
                                Self::get_agnt(&first.dst_addr));
                            return false;
                        }
                    };
                },
                Err(RecvTimeoutError::Disconnected) => {
                    debug!("Erro ao receber ACKS, thread Listener desconectada");
                    return false;
                },
            }
        }
        true
    }

    /// Currently, the friends are the next N nodes in the group vector, where N is the gossip rate
    fn get_friends(&self) -> Vec<SocketAddr> {
        let or_group = self.group
            .lock()
            .expect("Couldn't get grupo lock on get_friends");
        let mut group = Vec::new();
        let mut start = 0;
        for node in or_group.iter() {
            if !node.is_dead() {
                group.push(node.addr);
            }
            if node.agent_number == self.host.agent_number {
                start = group.len();
            }
        }

        let mut friends = Vec::new();
        for i in start..group.len() {
            if friends.len() < self.gossip_rate {
                friends.push(group[i]);
            }
            else {
                break;
            }
        }
        for i in 0..start {
            if friends.len() < self.gossip_rate {
                friends.push(group[i]);
            }
            else {
                break;
            }
        }
        friends
    }
}
