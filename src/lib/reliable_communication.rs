/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::packet::{Packet, HEADER_SIZE};
use crate::config::{Node, Broadcast, BROADCAST, BUFFER_SIZE, TIMEOUT, W_SIZE};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

pub struct ReliableCommunication {
    channel: Channel,
    pub host: Node,
    pub group: Vec<Node>,
    send_tx: Sender<(Sender<Packet>, SocketAddr, u32)>,
    receive_rx: Mutex<Receiver<Packet>>,
    msg_count: Mutex<HashMap<SocketAddr, usize>>,
    pkts_per_source: Mutex<HashMap<SocketAddr, Vec<u8>>>,
    msgs_per_source: Mutex<HashMap<SocketAddr, Vec<Vec<u8>>>>,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    /// Starts a new thread to listen for any incoming messages
    /// This thread will be responsible for handling the destination of each received packet
    pub fn new(host: Node, group: Vec<Node>) -> Self {
        let (send_tx, send_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let channel = match Channel::new(&host.addr) {
            Ok(c) => c,
            Err(e) => panic!("Erro {{{e}}} ao criar o canal de comunicação"),
        };
        channel.run(receive_tx, send_rx);
        let receive_rx = Mutex::new(receive_rx);
        Self {
            channel, host, group, send_tx, receive_rx,
            msg_count: Mutex::new(HashMap::new()),
            pkts_per_source: Mutex::new(HashMap::new()),
            msgs_per_source: Mutex::new(HashMap::new()),
        }
    }

    /// Send a message to a specific destination
    pub fn send(&self, dst_addr: &SocketAddr, message: &Vec<u8>) -> bool {
        let packets = self.get_packets(dst_addr, message, false);
        self.send_msg(packets) > 0
    }

    /// Read one already received message or wait for a message to arrive
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        let rcv = ||  self.receive_rx.lock().unwrap().recv_timeout(Duration::from_millis(TIMEOUT*100));
        let mut pkts = self.pkts_per_source.lock().unwrap();
        let mut msgs = self.msgs_per_source.lock().unwrap();
        loop {
            match rcv() {
                Ok(packet) => {if !packet.header.is_dlv() {
                    let Packet { header, data, .. } = packet;
                    pkts.entry(header.src_addr).or_insert(Vec::new()).extend(data);
                    if header.is_last() {
                        let msg = pkts.remove(&header.src_addr).unwrap();
                        if !header.r_dlv() {
                            buffer.extend(msg);
                            return true
                        }
                        msgs.entry(header.src_addr).or_insert(Vec::new()).push(msg);
                    }
                } else {
                    // pops a message from the buffer
                    if let Some(msgs) = msgs.get_mut(&packet.header.src_addr) {
                        let msg = msgs.remove(0);
                        buffer.extend(msg);
                        return true
                    }
                }}
                Err(RecvTimeoutError::Timeout) => {
                    debug_println!("Timeout ao receber mensagem");
                    return false
                },
                Err(RecvTimeoutError::Disconnected) => {
                    debug_println!("Erro ao receber pacote: Canal de comunicação desconectado");
                    return false
                },
            }
        }
    }

    /// Broadcasts a message, reliability level may be configured in the config file
    pub fn broadcast(&self, message: Vec<u8>) -> u32 {
        match BROADCAST {
            Broadcast::NONE => {
                let idx = (self.host.agent_number + 1) as usize % self.group.len();
                self.send(&self.group[idx].addr, &message) as u32
            },
            Broadcast::BEB => self.beb(message, &self.group, false).0.len() as u32,
            Broadcast::URB => self.urb(message),
            Broadcast::AB => self.ab(message),
        }
    }

    /// Fragments a message into packets
    fn get_packets(&self, dst_addr: &SocketAddr, message: &Vec<u8>, r_dlv: bool) -> Vec<Packet> {
        let chunks: Vec<&[u8]> = message.chunks(BUFFER_SIZE-HEADER_SIZE).collect();
        let start_packet = {
            let mut msg_count = self.msg_count.lock().unwrap();
            let count = msg_count.entry(*dst_addr).or_insert(0);
            let start = *count;
            *count += chunks.len() + (if r_dlv {1} else {0});
            start
        };

        chunks.iter().enumerate().map(|(i, pkt)| {
            Packet::new(
                self.host.addr,
                *dst_addr,
                (start_packet + i) as u32,
                None,
                i == (chunks.len() - 1),
                false,
                false,
                r_dlv,
                pkt.to_vec(),
            )
        }).collect()     
    }

    fn send_window(&self, next_seq_num: &mut usize, base: usize, packets: &Vec<Packet>) {
        while *next_seq_num < base + W_SIZE && *next_seq_num < packets.len() {
            self.channel.send(&packets[*next_seq_num]);
            *next_seq_num += 1;
        }
    }

    /// Algorithm for reliable 1:1 communication
    // tem também um parâmetro com valor default 0 para identificar o tipo de transmissão
    fn send_msg(&self, packets: Vec<Packet>) -> u32 {
        // Comunicação com a camada de canais
        let start_packet = packets[0].header.seq_num;
        let (ack_tx, ack_rx) = mpsc::channel();
        self.send_tx.send((ack_tx, packets[0].header.dst_addr, start_packet))
        .expect(format!("Erro ao inscrever o Agente {} para mandar pacotes", self.host.agent_number).as_str());

        // Algoritmo Go-Back-N para garantia de entrega dos pacotes
        let mut base = 0;
        let mut next_seq_num = 0;
        while base < packets.len() {
            // Envia todos os pacotes dentro da janela
            self.send_window(&mut next_seq_num, base, &packets);
            // Espera por um ACK
            match ack_rx.recv_timeout(Duration::from_millis(TIMEOUT)) {
                Ok(packet) => {
                    // assume que a listener está enviando o número do maior pacote que recebeu
                    // listener também garante que o pacote seja >= base
                    base = (packet.header.seq_num - start_packet + 1) as usize;
                },
                Err(RecvTimeoutError::Timeout) => {
                    // debug_println!("Timeout ao enviar pacote");
                    next_seq_num = base;
                },
                Err(RecvTimeoutError::Disconnected) => {
                    debug_println!("Erro ao enviar pacote: Canal de comunicação desconectado");
                    return start_packet + next_seq_num as u32
                },
            }
        }
        start_packet + next_seq_num as u32
    }

    /// Best-Effort Broadcast: attempts to send a message to all nodes in the group and return how many were successful
    /// This algorithm does not garantee delivery to all nodes if the sender fails
    fn beb(&self, message: Vec<u8>, group: &Vec<Node>, r_dlv: bool) -> (Vec<Node>, Vec<u32>) {
        let mut lst_seq_num = Vec::new();
        let new_group = group.iter()
            .filter(|node| {
                let pkts = self.get_packets(&node.addr, &message, r_dlv);
                lst_seq_num.push(self.send_msg(pkts));
                *lst_seq_num.last().unwrap() > 0
            })
            .cloned()
            .collect();
        (new_group, lst_seq_num)
    }

    /// Uniform Reliable Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all nodes receive the message if the sender does not fail
    fn urb(&self, message: Vec<u8>) -> u32 {
        let (group, lst_seq) = self.beb(message, &self.group, true);
        if group.len() >= self.group.len()*2 / 3 {
            for (i, node) in group.iter().enumerate() {
                self.send_dlv(&node.addr, lst_seq[i]);
            }
            return group.len() as u32
        }
        0
    }

    /// Atomic Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all messages are delivered in the same order to all nodes
    fn ab(&self, message: Vec<u8>) -> u32 {
        self.urb(message)
    }

    fn send_dlv(&self, dst_addr: &SocketAddr, lst_seq: u32) -> bool {
        let packet = Packet::new(
            self.host.addr,
            *dst_addr,
            lst_seq,
            None,
            true,
            false,
            true,
            false,
            Vec::new(),
        );
        // self.channel.send(&packet)
        self.send_msg(vec![packet]) as u32 > 0
    }
}

