use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

use logger::debug;
use crate::failure_detection::FailureDetection;
use crate::rec_aux::{SendRequest, Broadcast, RecAux};
use crate::channels::Channel;
use crate::packet::Packet;
use crate::node::Node;

/// Listener thread that handles the reception of messages
pub struct RecListener {
    host: Node,
    group: Arc<Mutex<Vec<Node>>>,
    channel: Arc<Channel>,
    broadcast: Broadcast,
    reg_to_snd_tx: Sender<SendRequest>,
}

impl RecAux for RecListener {}

impl RecListener {
    /// Constructor
    pub fn new(
        host: Node,
        group: Arc<Mutex<Vec<Node>>>,
        channel: Arc<Channel>,
        broadcast: Broadcast,
        reg_to_snd_tx: Sender<SendRequest>,
    ) -> Self {
        Self {
            host,
            group,
            channel,
            broadcast,
            reg_to_snd_tx,
        }
    }

    /// Thread to handle the reception of messages
    pub fn run(&self,
        messages_tx: Sender<Vec<u8>>,
        snd_acks_tx: Sender<Packet>,
        brd_acks_tx: Sender<Packet>,
        reg_snd_rx: Receiver<((SocketAddr, SocketAddr), u32)>,
        reg_brd_rx: Receiver<((SocketAddr, SocketAddr), u32)>,
        hb_tx: Sender<Packet>,
        brd_waiters_rx: Receiver<Sender<Vec<u8>>>,
    ) {
        let mut snd_pkts_per_origin: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut brd_pkts_per_origin: HashMap<SocketAddr, Vec<Packet>> = HashMap::new();
        let mut expected_snd_acks: HashMap<(SocketAddr, SocketAddr), u32> = HashMap::new();
        let mut expected_brd_acks: HashMap<(SocketAddr, SocketAddr), u32> = HashMap::new();
        let mut broadcast_waiters: Vec<Option<Sender<Vec<u8>>>> = Vec::new();
        loop {
            let packet = match self.channel.receive() {
                Ok(packet) => {packet},
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
            if packet.header.is_heartbeat() {
                FailureDetection::handle_hb(&packet, &self.group);
                match hb_tx.send(packet) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("Erro ao enviar heartbeat: {e}");
                    }
                }
                continue;
            } else if packet.header.is_ack() {
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
                            }
                        }
                    }
                    None => {
                        debug!("ACK recebido sem destinatário esperando");
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
                    // debug!("expected seq_num {expected}, recebeu {packet}");
                    continue;
                }
                // Send ack otherwise
                self.channel.send(&packet.get_ack());

                if packet.header.seq_num < expected {
                    continue;
                }

                if packet.header.is_last() {
                    let (message, origin, sequence_number) =
                        Self::receive_last_packet(&self, packets, &packet);
                    // Handling broadcasts
                    let dlv: bool = if packet.header.is_brd() {
                        match self.broadcast {
                            // BEB: All broadcasts must be delivered
                            Broadcast::BEB => {true}
                            // URB and AB: All broadcasts must be gossiped and then delivered
                            // those who are waiting for the broadcast must be warned
                            Broadcast::URB => {
                                Self::warn_brd_waiters(&mut broadcast_waiters, &brd_waiters_rx, &message);
                                Self::gossip(&self.reg_to_snd_tx, message.clone(), origin, sequence_number);
                                true
                            },
                            Broadcast::AB => {
                                Self::warn_brd_waiters(&mut broadcast_waiters, &brd_waiters_rx, &message);
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
                                debug!("Erro ao enviar mensagem: {e}");
                            }
                        }
                    }
                }
                debug!(">>> pushing {packet} on the packets buffer");
                packets.push(packet);
            }
        }
    }

    /// Decides what to do with a broadcast message in the AB algorithm
    /// Based on your priority and the priority of the origin of the message
    /// The return boolean is used to tell the listener thread whether the message should be delivered or not (in case it's a broadcast request for the leader)
    fn atm_gossip(&self, message: Vec<u8>, origin: &SocketAddr,
            sequence_number: u32) -> bool {
        let origin_priority = self.get_leader_priority(&origin);
        let own_priority = self.get_leader_priority(&self.host.addr);
        if origin_priority < own_priority {
            // If the origin priority is lower than yours, it means the the origin considers you the leader and you must broadcast the message
            debug!("Recebeu um Leader Request de {}", Self::get_agnt(origin));
            Self::brd_req(&self.reg_to_snd_tx, message.clone());
            false
        } else {
            // If the origin priority is higher or equal to yours,
            // it means the origin is the leader and you must simply gossip the message
            Self::gossip(&self.reg_to_snd_tx, message.clone(), *origin, sequence_number);
            true
        }
    }

    /// When a packet marked as last is received, the packets are merged and the message is returned
    fn receive_last_packet(&self, packets: &mut Vec<Packet>,
        packet: &Packet) -> (Vec<u8>, SocketAddr, u32) {
        // Ignore the first packet if its the remnant of a previous message
        match packets.first() {
            Some(p) => {
                if p.header.is_last() {
                    debug!("> Ignorando pacote remanescente de uma mensagem anterior");
                    packets.remove(0);
                }
            }
            None => {}
        }

        let mut message = Vec::new();
        for packet in packets.iter() {
            message.extend(&packet.data);
        }
        message.extend(&packet.data);

        let p = match packets.first() {
            Some(p) => { p }
            None => { &packet }
        };
        let seq_num: u32 = p.header.seq_num;
        packets.clear();

        (message, packet.header.origin, seq_num)
    }

    /// Resends the message for anyone who is waiting for a broadcast
    fn warn_brd_waiters(brd_waiters: &mut Vec<Option<Sender<Vec<u8>>>>,
        brd_waiters_rx: &Receiver<Sender<Vec<u8>>>, message: &Vec<u8>) {
        while let Ok(brd_waiter) = brd_waiters_rx.try_recv() {
            brd_waiters.push(Some(brd_waiter));
        }
        for waiter in brd_waiters.iter_mut() {
            let w = waiter.as_ref().unwrap();
            match (*w).send(message.clone()) {
                Ok(_) => {} 
                Err(_) => {
                    *waiter = None;
                }
            }
        }
        brd_waiters.retain(|w| w.is_some());
    }

    /// Calculates the priority of a node in the group (currently the lowest index in the group vector)
    fn get_leader_priority(&self, node_address: &SocketAddr) -> usize {
        let group = self.group
            .lock()
            .expect("Erro ao obter prioridade do líder: Mutex lock do grupo falhou");
        for (i, n) in group.iter().enumerate() {
            if n.addr == *node_address {
                return group.len()-i;
            }
        }
        0
    }
}
