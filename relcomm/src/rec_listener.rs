use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::collections::HashMap;

use crate::node::Node;
use crate::channels::Channel;
use crate::packet::Packet;
use crate::rec_aux::{SendRequest, Broadcast, RecAux};
use logger::debug;
use logger::log::{PacketStatus, SharedLogger};

/// Listener thread that handles the reception of messages
pub struct RecListener {
    host: Node,
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
        snd_acks_tx: Sender<Packet>,
        brd_acks_tx: Sender<Packet>,
        reg_snd_rx: Receiver<((SocketAddr, SocketAddr), u32)>,
        reg_brd_rx: Receiver<((SocketAddr, SocketAddr), u32)>,
        brd_waiters_rx: Receiver<Sender<Vec<u8>>>,
    ) {
        let mut snd_pkts_per_origin: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut brd_pkts_per_origin: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut expected_snd_acks: HashMap<(SocketAddr, SocketAddr), u32> = HashMap::new();
        let mut expected_brd_acks: HashMap<(SocketAddr, SocketAddr), u32> = HashMap::new();
        let mut broadcast_waiters: Vec<Option<Sender<Vec<u8>>>> = Vec::new();
        loop {
            let packet = match self.channel.receive() {
                Ok(packet) => packet,
                Err(e) => {
                    debug!("Falhou ao receber um pacote do socket, erro: {e}");
                    continue;
                }
            };
            let (reg, expected_acks, acks_tx, pkts_per_origin) = if packet.header.is_brd() {
                (&reg_brd_rx, &mut expected_brd_acks, &brd_acks_tx, &mut brd_pkts_per_origin)
            } else {
                (&reg_snd_rx, &mut expected_snd_acks, &snd_acks_tx, &mut snd_pkts_per_origin)
            };
            if packet.header.is_ack() {
                // logger
                Self::log_pkt(&self.logger, &self.host, &packet, PacketStatus::ReceivedAck);

                // Handle ack
                while let Ok((key, start_seq)) = reg.try_recv() {
                    expected_acks.insert(key, start_seq);
                }
                match expected_acks.get_mut(&(packet.header.src_addr, packet.header.origin)) {
                    Some(seq_num) => {
                        if packet.header.seq_num < *seq_num { continue; }
                        *seq_num = packet.header.seq_num + 1;
                        match acks_tx.send(packet.clone()) {
                            Ok(_) => {}
                            Err(e) => {
                                debug!("Erro ao enviar ACK: {e}");
                                // logger
                                Self::log_pkt(&self.logger, &self.host, &packet, PacketStatus::ReceivedAckFailed);
                            }
                        }
                    }
                    None => {
                        debug!("ACK recebido sem destinatário esperando");
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
                    let dlv: bool = if packet.header.is_brd() {
                        debug!("Recebeu um broadcast do Agente {}", origin.port() % 100);
                        match self.broadcast {
                            // BEB: All broadcasts must be delivered
                            Broadcast::BEB => {}
                            // URB: All broadcasts must be gossiped and then delivered
                            Broadcast::URB => {
                                Self::gossip(&self.register_to_sender_tx, message.clone(), origin, sequence_number);
                            }
                            // AB: Must check if I'm the leader and should broadcast it or just gossip
                            // In AB, broadcast messages can only be delivered if they were sent by the leader
                            // However, since the leader is the only one that can broadcast,
                            // this is only useful for the leader,
                            // who must not deliver the request to broadcast to the group,
                            // instead, it must broadcast the message and only deliver when it gets gossiped back to it
                            Broadcast::AB => {
                                Self::warn_brd_waiters(&mut broadcast_waiters, &brd_waiters_rx, &message);
                                Self::gossip(&self.register_to_sender_tx, message.clone(), origin, sequence_number);
                            }
                        }
                        true
                    } else {
                        // TODO: FIX this breaking the 1:1 Sends
                        // Create a way to diferentiate between a leader request and a normal message
                        let origin_priority = self.get_leader_priority(&origin);
                        let own_priority = self.get_leader_priority(&self.host.addr);
                        if origin_priority < own_priority {
                            // If the origin priority is lower than yours, it means the the origin considers you the leader and you must broadcast the message
                            debug!("É Líder e recebeu um pedido de {}", origin.port() % 100);
                            Self::brd_req(&self.register_to_sender_tx, message.clone());
                            false
                        } else {true}
                    };
                    if dlv {
                        match messages_tx.send(message) {
                            Ok(_) => {}
                            Err(e) => {
                                debug!("Erro ao enviar mensagem: {e}");
                            }
                        }
                    }
                }
                packets.push(packet);
            }
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

    /// Resends the message for anyone who is waiting for a broadcast
    fn warn_brd_waiters(broadcast_waiters: &mut Vec<Option<Sender<Vec<u8>>>>, brd_waiters_rx: &Receiver<Sender<Vec<u8>>>, message: &Vec<u8>) {
        while let Ok(broadcast_waiter) = brd_waiters_rx.try_recv() {
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
}
