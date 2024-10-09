/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::packet::{Packet, HEADER_SIZE};
use crate::config::{Node, BUFFER_SIZE, TIMEOUT, W_SIZE, TIMEOUT_LIMIT};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;

pub struct ReliableCommunication {
    channel: Channel,
    pub host: SocketAddr,
    pub group: Vec<Node>,
    send_tx: mpsc::Sender<(Sender<Packet>, SocketAddr, u32)>,
    receive_rx: Mutex<Receiver<Packet>>,
    msg_count: Mutex<HashMap<SocketAddr, u32>>,
    message_per_source: Mutex<HashMap<SocketAddr, Vec<u8>>>,
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: SocketAddr, group: Vec<Node>) -> Self {
        let (send_tx, send_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let channel = match Channel::new(host.clone()) {
            Ok(c) => c,
            Err(e) => panic!("Erro {{{e}}} ao criar o canal de comunicação"),
        };
        channel.run(receive_tx, send_rx);
        let receive_rx = Mutex::new(receive_rx);
        
        Self { channel, host, group, send_tx, receive_rx,
            msg_count: Mutex::new(HashMap::new()),
            message_per_source: Mutex::new(HashMap::new()) }
    }

    fn prepare_to_send(&self, dst_addr: &SocketAddr, packets_len: u32)
                        -> (usize, Receiver<Packet>) {
        let start_packet = {
            let mut msg_count = self.msg_count.lock().unwrap();
            let count = msg_count.entry(*dst_addr).or_insert(0);
            let start = *count;
            *count += packets_len;
            start as usize
        };
        
        // Comunicação com a camada de canais
        let (ack_tx, ack_rx) = mpsc::channel();
        let agente = self.host.port() % 100;
        self.send_tx.send((ack_tx, *dst_addr, start_packet as u32))
        .expect(format!("Erro ao inscrever o Agente {agente} para mandar pacotes").as_str());
        (start_packet, ack_rx)        
    }

    fn send_window(&self, next_seq_num: &mut usize, base: usize,
                    packets: &Vec<&[u8]>, start_packet: usize,
                    dst_addr: &SocketAddr ) {
        while *next_seq_num < base + W_SIZE && *next_seq_num < packets.len() {
            let packet = Packet::new(
                self.host,
                *dst_addr,
                (*next_seq_num + start_packet) as u32,
                None,
                *next_seq_num == packets.len() - 1,
                false,
                packets[*next_seq_num].to_vec(),
            );
            self.channel.send(packet);
            *next_seq_num += 1;
        }
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> bool {
        // Preparar a mensagem para ser enviada
        let packets: Vec<&[u8]> = message.chunks(BUFFER_SIZE-HEADER_SIZE).collect();

        let (start_packet, ack_rx) = self.prepare_to_send(dst_addr,
            packets.len() as u32);
        
        // Algoritmo Go-Back-N para garantia de entrega dos pacotes
        let mut count_timeout = 0;
        let mut base = 0;
        let mut next_seq_num = 0;
        while base < packets.len() {
            // Envia todos os pacotes dentro da janela
            self.send_window(&mut next_seq_num, base, &packets, start_packet, dst_addr);
            // Espera por um ACK
            match ack_rx.recv_timeout(std::time::Duration::from_millis(TIMEOUT)) {
                Ok(packet) => {
                    count_timeout = 0;
                    // assume que a listener está enviando o número do maior pacote que recebeu
                    // listener também garante que o pacote seja >= base
                    base = packet.header.seq_num as usize - start_packet + 1;
                },
                Err(RecvTimeoutError::Timeout) => {
                        count_timeout += 1;
                        next_seq_num = base;
                        if count_timeout == TIMEOUT_LIMIT {
                            return false
                        }
                    },
                Err(RecvTimeoutError::Disconnected) => return true,
            }
        }
        true
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        let rcv = || {
            match cfg!(debug_assertions) {
                true => self.receive_rx.lock().unwrap()
                .recv_timeout(std::time::Duration::from_millis(TIMEOUT*100)),
                false => self.receive_rx.lock().unwrap()
                .recv().map_err(|_| mpsc::RecvTimeoutError::Timeout)
            }
        };
        let mut messages = self.message_per_source.lock().unwrap();
        while let Ok(packet) = rcv() {
            let Packet { header, data, .. } = packet;
            messages.entry(header.src_addr).or_insert(Vec::new()).extend(data);
            if header.is_last() {
                let msg = messages.remove(&header.src_addr).unwrap();
                buffer.extend(msg);
                return true
            }
        }
        false
    }

    fn beb(&self, message: Vec<u8>) -> bool {
        let mut success = 0;
        for node in &self.group {
            if !self.send(&node.addr, message.clone()) {
                success += 1;
            }
        }
        let max = self.group.len();
        success >= max*(2/3)
    }

    pub fn broadcast(&self, message: Vec<u8>) -> bool {
        self.beb(message)
    }
}

