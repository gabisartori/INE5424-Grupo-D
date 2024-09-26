/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::packet::{Packet, HEADER_SIZE};
use crate::config::{Node, BUFFER_SIZE, TIMEOUT, W_SIZE, TIMEOUT_LIMIT};
use super::message_queue::MessageQueue;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Mutex, Arc};

pub struct ReliableCommunication {
    channel: Channel,
    pub host: SocketAddr,
    pub group: Vec<Node>,
    send_tx: mpsc::Sender<(Arc<MessageQueue<Packet>>, SocketAddr, u32)>,
    receive_rx: Mutex<Receiver<Packet>>,
    // uma variável compartilhada (Arc<Mutex>) que conta quantas vezes send foi chamada
    msg_count: Mutex<HashMap<SocketAddr, u32>>,
    message_per_source: Mutex<HashMap<SocketAddr, Vec<u8>>>
}

// TODO: Fazer com que a inicialização seja de um grupo
impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: SocketAddr, group: Vec<Node>) -> Self {
        let (send_tx, send_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let channel = match Channel::new(host.clone(), send_rx, receive_tx) {
            Ok(c) => c,
            Err(_) => panic!("Erro ao criar o canal de comunicação"),
        };
        let receive_rx =Mutex::new(receive_rx);
        
        Self { channel, host, group, send_tx, receive_rx, msg_count: Mutex::new(HashMap::new()), message_per_source: Mutex::new(HashMap::new()) }
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> bool {
        // Preparar a mensagem para ser enviada
        let packets: Vec<&[u8]> = message.chunks(BUFFER_SIZE-HEADER_SIZE).collect();
        let start_packet: usize = {
            let mut msg_count = self.msg_count.lock().unwrap();
            let count = msg_count.entry(*dst_addr).or_insert(0);
            let start = *count;
            *count += packets.len() as u32;
            start as usize
        };
        
        // Comunicação com a camada de canais
        // let (ack_tx, ack_rx) = mpsc::channel();
        let ack_rx = std::sync::Arc::new(MessageQueue::new());
        let agente = self.host.port() % 100;
        self.send_tx.send((ack_rx.clone(), *dst_addr, start_packet as u32))
        .expect(format!("Erro ao inscrever o Agente {agente} para mandar pacotes").as_str());
        
        // Algoritmo Go-Back-N para garantia de entrega dos pacotes
        let mut count_timeout = 0;
        let mut base = 0;
        let mut next_seq_num = 0;
        while base < packets.len() {
            // Envia todos os pacotes dentro da janela
            while next_seq_num < base + W_SIZE && next_seq_num < packets.len() {
                let packet = Packet::new(
                    self.host,
                    *dst_addr,
                    (next_seq_num + start_packet) as u32,
                    None,
                    next_seq_num == packets.len() - 1,
                    false,
                    packets[next_seq_num].to_vec(),
                );
                self.channel.send(packet);
                next_seq_num += 1;
            }
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
                true => self.receive_rx.lock().unwrap().recv_timeout(std::time::Duration::from_millis(10*TIMEOUT)),
                false => self.receive_rx.lock().unwrap().recv().map_err(|_| mpsc::RecvTimeoutError::Timeout)
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
}

