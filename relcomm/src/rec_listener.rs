use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::collections::HashMap;

use crate::node::Node;
use crate::channels::Channel;
use crate::packet::Packet;
use crate::rec_aux::{SendRequest, Broadcast, RecAux};
use logger::debug_println;
use logger::log::{PacketStatus, SharedLogger};

// TODO: reduce the need for self parameters by separating elements that don't need to be shared 
/// Listener thread that handles the reception of messages
pub struct RecListener {
    host: Node,
    // TODO: Make a global broadcast counter
    group: Arc<Mutex<Vec<Node>>>,
    channel: Arc<Channel>,
    broadcast: Broadcast,
    logger: SharedLogger,
    register_to_sender_tx: Sender<SendRequest>,
}

impl RecAux for RecListener {}

impl RecListener {
    /// Constructor
    pub fn new(
        host: Node,
        group: Arc<Mutex<Vec<Node>>>,
        channel: Arc<Channel>,
        broadcast: Broadcast,
        logger: SharedLogger,
        register_to_sender_tx: Sender<SendRequest>,
    ) -> Self {
        Self {
            host,
            group,
            channel,
            broadcast,
            logger,
            register_to_sender_tx,
        }
    }

    /// Thread to handle the reception of messages
    pub fn run(&self,
        messages_tx: Sender<Vec<u8>>,
        acks_tx: Sender<Packet>,
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
            if packet.header.is_ack() {
                // logger
                Self::log_pkt(&self.logger, &self.host, &packet, PacketStatus::ReceivedAck);

                // Handle ack
                while let Ok((key, start_seq)) = register_from_sender_rx.try_recv() {
                    expected_acks.insert(key, start_seq);
                }
                match expected_acks.get_mut(&(packet.header.src_addr, packet.header.origin)) {
                    Some(seq_num) => {
                        if packet.header.seq_num < *seq_num { continue; }
                        *seq_num = packet.header.seq_num + 1;
                        match acks_tx.send(packet.clone()) {
                            Ok(_) => {}
                            Err(e) => {
                                debug_println!("Erro ao enviar ACK: {e}");
                                // logger
                                Self::log_pkt(&self.logger, &self.host, &packet, PacketStatus::ReceivedAckFailed);
                            }
                        }
                    }
                    None => {
                        debug_println!("ACK recebido sem destinatário esperando");
                        // logger
                        Self::log_pkt(&self.logger, &self.host, &packet, PacketStatus::ReceivedAckFailed);
                    }
                }
            } else {
                // logger
                Self::log_pkt(&self.logger, &self.host, &packet, PacketStatus::Received);

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
                // logger
                Self::log_pkt(&self.logger, &self.host, &packet, PacketStatus::SentAck);

                if packet.header.seq_num < expected { continue; }
                
                if packet.header.is_last() {
                    let (message, origin, sequence_number) =
                        Self::receive_last_packet(&self, packets, &packet);
                    // Handling broadcasts
                    let dlv: bool = if packet.header.must_gossip() {
                        match self.broadcast {
                            // BEB: All broadcasts must be delivered
                            Broadcast::BEB => {
                                true
                            }
                            // URB: All broadcasts must be gossiped and then delivered
                            Broadcast::URB => {
                                Self::gossip(&self.register_to_sender_tx, message.clone(), origin, sequence_number);
                                true
                            }
                            // AB: Must check if I'm the leader and should broadcast it or just gossip
                            // In AB, broadcast messages can only be delivered if they were sent by the leader
                            // However, since the leader is the only one that can broadcast,
                            // this is only useful for the leader,
                            // who must not deliver the request to broadcast to the group,
                            // instead, it must broadcast the message and only deliver when it gets gossiped back to it
                            Broadcast::AB => {
                                self.warn_brd_waiters(&mut broadcast_waiters, &register_broadcast_waiters_rx, message.clone());
                                
                                self.atm_gossip(message.clone(), &origin, sequence_number)
                            }
                        }
                    } else {
                        true
                    };
                    if dlv {
                        match messages_tx.send(message) {
                            Ok(_) => {}
                            Err(e) => {
                                debug_println!("Erro ao enviar mensagem: {e}");
                            }
                        }
                    }
                }
                packets.push(packet);
            }
        }
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

    /// Resends the message for anyone who is waiting for a broadcast
    fn warn_brd_waiters(&self, broadcast_waiters: &mut Vec<Option<Sender<Vec<u8>>>>, register_broadcast_waiters_rx: &Receiver<Sender<Vec<u8>>>, message: Vec<u8>) {
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
            Self::brd_req(&self.register_to_sender_tx, message);
            false
        } else {
            // If the origin priority is higher or equal to yours, it means the origin is the leader and you must simply gossip the message
            Self::gossip(&self.register_to_sender_tx, message, *origin, sequence_number);
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
                if p.header.is_last() { packets.remove(0); }
            }
            None => {}
        }

        for packet in packets.iter() {
            message.extend(&packet.data);
        }
        message.extend(&packet.data);

        let p = match packets.first() {
            Some(p) => { p
        }
            None => { &packet}
        };
        // logger
        Self::log_pkt(&self.logger, &self.host, &p, PacketStatus::ReceivedLastPacket);

        let seq_num: u32 = p.header.seq_num;
        packets.clear();

        (message, packet.header.origin, seq_num)
    }
}
