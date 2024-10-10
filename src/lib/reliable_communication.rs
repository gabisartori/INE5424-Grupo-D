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

pub struct ReliableCommunication {
    channel: Channel,
    pub host: Node,
    pub group: Vec<Node>,
    send_tx: Sender<(Sender<Packet>, SocketAddr, u32)>,
    receive_rx: Mutex<Receiver<Packet>>,
    msg_count: Mutex<HashMap<SocketAddr, u32>>,
    pkts_per_source: Mutex<HashMap<SocketAddr, Vec<u8>>>,
    msgs_per_source: Mutex<HashMap<SocketAddr, Vec<Vec<u8>>>>,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: Node, group: Vec<Node>) -> Self {
        let (send_tx, send_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let channel = match Channel::new(host.addr.clone()) {
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

    fn prepare_to_send(&self, dst_addr: &SocketAddr, message: Vec<u8>)
                        -> (usize, Vec<Packet>) {
        let chunks: Vec<&[u8]> = message.chunks(BUFFER_SIZE-HEADER_SIZE).collect();
        let packets_len = chunks.len();
        let start_packet = {
            let mut msg_count = self.msg_count.lock().unwrap();
            let count = msg_count.entry(*dst_addr).or_insert(0);
            let start = *count;
            *count += packets_len as u32;
            start as usize
        };
        let mut packets: Vec<Packet> = Vec::new();
        let mut i = 0;
        for pkt in chunks {
            packets.push(Packet::new(
                self.host.addr,
                *dst_addr,
                (i + start_packet) as u32,
                None,
                i == (packets_len - 1),
                false,
                false,
                pkt.to_vec(),
            ));
            i += 1;
        }
        (start_packet, packets)        
    }

    fn send_window(&self, next_seq_num: &mut usize,
                    base: usize,
                    packets: &Vec<Packet>) {
        while *next_seq_num < base + W_SIZE && *next_seq_num < packets.len() {
            self.channel.send(&packets[*next_seq_num].clone());
            *next_seq_num += 1;
        }
    }

    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> bool {

        // Preparar a mensagem para ser enviada
        let (start_packet, packets) = self.prepare_to_send(dst_addr, message);
        self.send_msg(packets, start_packet)
    }
    // Função para enviar mensagem com garantias de comunicação confiável
    // tem também um parâmetro com valor default 0 para identificar o tipo de transmissão
    pub fn send_msg(&self, packets: Vec<Packet>, start_packet: usize) -> bool {
        // Comunicação com a camada de canais

        let (ack_tx, ack_rx) = mpsc::channel();
        self.send_tx.send((ack_tx, packets[0].header.dst_addr, start_packet as u32))
        .expect(format!("Erro ao inscrever o Agente {} para mandar pacotes", self.host.agent_number).as_str());

        // Algoritmo Go-Back-N para garantia de entrega dos pacotes
        let mut base = 0;
        let mut next_seq_num = 0;
        while base < packets.len() {
            // Envia todos os pacotes dentro da janela
            self.send_window(&mut next_seq_num, base, &packets);
            // Espera por um ACK
            match ack_rx.recv_timeout(std::time::Duration::from_millis(TIMEOUT)) {
                Ok(packet) => {
                    // assume que a listener está enviando o número do maior pacote que recebeu
                    // listener também garante que o pacote seja >= base
                    base = packet.header.seq_num as usize - start_packet + 1;
                },
                Err(RecvTimeoutError::Timeout) => {next_seq_num = base;},
                Err(RecvTimeoutError::Disconnected) => return true,
            }
        }
        true
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        let rcv = ||  self.receive_rx.lock().unwrap()
                .recv_timeout(std::time::Duration::from_millis(TIMEOUT*100));
        let mut pkts = self.pkts_per_source.lock().unwrap();
        let mut msgs = self.msgs_per_source.lock().unwrap();
        while let Ok(packet) = rcv() {
            if !packet.header.is_dlv() {
                let Packet { header, data, .. } = packet;
                pkts.entry(header.src_addr).or_insert(Vec::new()).extend(data);
                if header.is_last() {
                    let msg = pkts.remove(&header.src_addr).unwrap();
                    if BROADCAST == Broadcast::NONE || BROADCAST == Broadcast::BEB {
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
            }
        }
        false
    }

    fn beb(&self, message: Vec<u8>, group: Vec<Node>) -> Vec<Node> {
        let mut success = Vec::new();
        
        for node in group {
            let (start_packet,
                pkts) = self.prepare_to_send(&node.addr, message.clone());
            if self.send_msg(pkts, start_packet) {
                success.push(node.clone());
            }
        }
        success
    }

    fn urb(&self, message: Vec<u8>) -> u32 {
        let group = self.beb(message, self.group.clone());
        if group.len() >= self.group.len()*2 / 3 {
            for node in group.iter() {
                self.send_dlv(&node.addr);
            }
            return group.len() as u32
        }
        0
    }

    fn ab(&self, message: Vec<u8>) -> u32 {
        self.urb(message)
    }

    pub fn broadcast(&self, message: Vec<u8>) -> u32 {
        match BROADCAST {
            Broadcast::NONE => {
                let idx = (self.host.agent_number + 1) as usize % self.group.len();
                self.send(&self.group[idx].addr, message) as u32
            },
            Broadcast::BEB => {
                let suc = self.beb(message, self.group.clone());
                suc.len() as u32
            },
            Broadcast::URB => self.urb(message),
            Broadcast::AB => self.ab(message),
        }
    }

    fn send_dlv(&self, dst_addr: &SocketAddr) -> bool {
        let packet = Packet::new(
            self.host.addr,
            *dst_addr,
            0,
            None,
            true,
            false,
            true,
            Vec::new(),
        );
        self.channel.send(&packet)
        // self.send_msg(vec![packet], 0)
    }
}

