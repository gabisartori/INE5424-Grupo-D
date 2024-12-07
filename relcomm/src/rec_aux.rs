use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use logger::{debug, log::{Logger, LoggerState, MessageStatus, PacketStatus}};
use crate::node::Node;
use crate::packet::Packet;

#[derive(Clone)]
pub enum SendRequestData {
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

#[derive(Clone)]
pub struct SendRequest {
    pub result_tx: Sender<u32>,
    pub data: Vec<u8>,
    pub options: SendRequestData,
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

#[derive(PartialEq, Clone)]
pub enum Broadcast {
    BEB,
    URB,
    AB,
}
/// This struct contains helper functions that are used by the main, listener and sender thread
pub trait RecAux {
    /// Creates a broadcast request and sends it to the sender thread
    fn brd_req(register_to_sender_tx: &Sender<SendRequest>, data: Vec<u8>) -> Receiver<u32>{
        debug!("Enviando broadcast");
        let (request, request_rx) = SendRequest::new(
            data,
            SendRequestData::StartBroadcast {},
        );
        match register_to_sender_tx.send(request) {
            Ok(_) => {}
            Err(e) => {
                debug!("Erro ao fazer broadcast: {e}");
            }
        }
        request_rx
    }

    /// Picks the node "friends" and retransmits the message to them
    /// This retransmission preserves the original message information about the origin and sequence number
    /// The friends are any group of N nodes in the group, where N is the gossip rate.
    /// Currently it's the next N nodes in the group vector
    /// Since gossip algorithms are meant to ensure that the message will be successfully difused,
    /// even if there are failing nodes, this function doesn't need to wait for the result of the gossip.
    /// (It's also important to not block the listener thread when it needs to gossip a message)
    fn gossip(register_to_sender_tx: &Sender<SendRequest>, data: Vec<u8>, origin: SocketAddr, seq_num: u32) {
        let (request, _) = SendRequest::new(
            data,
            SendRequestData::Gossip {
                origin,
                seq_num,
            },
        );
        match register_to_sender_tx.send(request) {
            Ok(_) => {}
            Err(e) => {
                debug!("Erro ao fazer fofoca: {e}");
            }
        }
    }

    /// Returns the node with the highest priority (currently the first one alive in the group vector)
    fn get_leader(group: &Arc<Mutex<Vec<Node>>>, host: &Node) -> Node {
        for node in group.lock().expect("Falha ao ler do grupo").iter() {
            if !node.is_dead() {
                // debug!("Agente {} escolheu {} como líder", host.agent_number, node.agent_number);
                return node.clone();
            }
        }
        return host.clone();
    }

    /// Returns a vector with all nodes that are not dead
    fn get_livings(group: &Arc<Mutex<Vec<Node>>>) -> Vec<Node> {
        let mut livings = Vec::new();
        for node in group.lock().expect("Falha ao ler do grupo").iter() {
            if !node.is_dead() {
                livings.push(node.clone());
            }
        }
        livings
    }

    fn get_agnt(addr: &SocketAddr) -> usize {
        addr.port() as usize % 100
    }

    fn log_msg(logger: &Arc<Logger>, host: &Node, pkt: &Packet, state: MessageStatus) {
        let other_id = if host.addr == pkt.header.src_addr {
            Self::get_agnt(&pkt.header.dst_addr)
        } else {
            Self::get_agnt(&pkt.header.src_addr)
        };
        let logger_state = LoggerState::Message {
            state,
            current_agent_id: Some(host.agent_number),
            target_agent_id: Some(other_id),
            message_id: pkt.header.seq_num as usize,
        };
        logger.log(logger_state);
    }

    fn log_pkt(logger: &Arc<Logger>, host: &Node, pkt: &Packet, state: PacketStatus) {
        let other_id = if host.addr == pkt.header.src_addr {
            Self::get_agnt(&pkt.header.dst_addr)
        } else {
            Self::get_agnt(&pkt.header.src_addr)
        };
        let logger_state = LoggerState::Packet {
            state,
            current_agent_id: Some(host.agent_number),
            target_agent_id: Some(other_id),
            seq_num: pkt.header.seq_num as usize,
        };

        logger.log(logger_state);
    }

    fn send_nonblocking(reg_to_snd_tx: &Sender<SendRequest>,
        dst_addr: &SocketAddr, msg: Vec<u8>) -> Receiver<u32> {
        let (request, result_rx) = SendRequest::new(
            msg,
            SendRequestData::Send {
                destination_address: *dst_addr,
            },
        );
        match reg_to_snd_tx.send(request) {
            Ok(_) => {}
            Err(e) => {
                debug!("Erro ao registrar request: {e}");
            }
        }
        result_rx
    }
}
