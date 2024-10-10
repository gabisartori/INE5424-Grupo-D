/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::{UdpSocket, SocketAddr};
use std::io::Error;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::config::{BUFFER_SIZE, LOSS_RATE};
use super::packet::Packet;
// use super::failure_detection;

// Estrutura básica para a camada de comunicação por canais
#[derive(Clone)]
pub struct Channel {
    socket: std::sync::Arc<UdpSocket>,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(bind_addr: &SocketAddr) -> Result<Self, Error> {
        let socket = std::sync::Arc::new(UdpSocket::bind(bind_addr)?);
        Ok(Self { socket })
    }
    
    pub fn run(&self,
                tx_msgs: Sender<Packet>,
                send_rx: Receiver<(Sender<Packet>, SocketAddr, u32)>) {
        let clone = self.clone();
        thread::spawn(move || {
            Channel::listener(&clone, tx_msgs, send_rx);
        });
    }

    fn listener(&self,
                tx_msgs: Sender<Packet>,
                rx_acks: Receiver<(Sender<Packet>, SocketAddr, u32)>) {
        let mut sends: HashMap<SocketAddr, (Sender<Packet>, u32)> = HashMap::new();
        let mut messages_sequence_numbers: HashMap<SocketAddr, u32> = HashMap::new();
        loop {
            let packet;
            match self.receive() {
                Some(p) => packet = p,
                None => continue 
            };
            // Verifica se o pacote recebido é válido
            if !Channel::validate_message(&packet) {continue;}
            
            if packet.header.is_ack() {
                Channel::process_acks(packet, &mut sends, &rx_acks);
            } else if packet.header.is_dlv() {
                // Encaminhar o pacote para a fila de mensagens
                Channel::deliver(&tx_msgs, packet);
            } else {
                // Verificar se o pacote é o próximo esperado
                let next_seq_num = *messages_sequence_numbers.get(&packet.header.src_addr).unwrap_or(&0);
                if packet.header.seq_num > next_seq_num {continue;}
                // Send ACK
                if !self.send(&packet.get_ack()) {continue;}
                // Encaminhar o pacote para a fila de mensagens se for o próximo esperado
                if packet.header.seq_num < next_seq_num { continue; }
                messages_sequence_numbers.insert(packet.header.src_addr, packet.header.seq_num + 1);
                Channel::deliver(&tx_msgs, packet);
            }
        }
    }

    fn validate_message(packet: &Packet) -> bool {
        // Checksum
        let c1: bool = packet.header.checksum == Packet::checksum(&packet.header, &packet.data);
        c1
    }

    fn process_acks(packet: Packet,
                    sends: &mut HashMap<SocketAddr, (Sender<Packet>, u32)>,
                    rx_acks: &Receiver<(Sender<Packet>, SocketAddr, u32)>) {
        // Verifica se há alguém esperando pelo ACK recebido
        while let Ok((tx, key, start_seq)) = rx_acks.try_recv() {
            match sends.entry(key) {
                Entry::Occupied(mut entry) => {
                    let (tx_, seq_num) = entry.get_mut();
                    if *seq_num == start_seq {
                        *tx_ = tx;
                    } else {
                        *tx_ = tx;
                    }
                },
                Entry::Vacant(entry) => {
                    entry.insert((tx, start_seq));
                }
            }
        }
        // Se houver, encaminha o ACK para o remetente
        // Senão, ignora o ACK
        if let Some((tx, seq_num)) = sends.get_mut(&packet.header.src_addr) {
            if packet.header.seq_num < *seq_num {
                return;
            }
            *seq_num = packet.header.seq_num + 1;
            Channel::deliver(tx, packet);
            // let is_last = packet.is_last();
            // let src = packet.header.src_addr;
            // if is_last {
            //     sends.remove(&src);
            // }
        }
    }

    fn deliver(tx: &Sender<Packet>, packet: Packet) {
        let agent_s = packet.header.src_addr.port() % 100;
        let agent_d = packet.header.dst_addr.port() % 100;
        let pk = packet.header.seq_num;
        let is_ack = packet.header.is_ack();
        match tx.send(packet) {
            Ok(_) => (),
            Err(e) => {
                if is_ack {
                    debug_println!("->-> Erro {{{e}}} ao entregar ACK {pk} do Agente {agent_s} para o Agente {agent_d} pelo canal");
                } else {
                    debug_println!("->-> Erro {{{e}}} ao entregar pacote {pk} do Agente {agent_s} para o Agente {agent_d} pelo canal");
                }
            }
        }
    }

    fn receive(&self) -> Option<Packet> {
        let mut buffer = [0; BUFFER_SIZE];
        let size;
        match self.socket.recv_from(&mut buffer) {
            Ok((size_, _)) => { size = size_; },
            Err(e) => {
                let agent = self.socket.local_addr().unwrap().port() % 100;
                debug_println!("->-> Erro {{{e}}} no Agente {agent} ao receber pacote pelo socket");
                return None;
            },
        }
        // Simula perda de pacotes
        if rand::random::<f32>() < LOSS_RATE {
            return None;
        }
        Some(Packet::from_bytes(buffer, size))

    }

    pub fn send(&self, packet: &Packet) -> bool { 
        let agent_s = packet.header.src_addr.port() % 100;
        let agent_d = packet.header.dst_addr.port() % 100;
        let pk = packet.header.seq_num;
        let is_ack = packet.header.is_ack();
        match self.socket.send_to(&packet.to_bytes(), packet.header.dst_addr) {
            Ok(_) => true,
            Err(e) => {
                if is_ack {
                    debug_println!("->-> Erro {{{e}}} ao enviar ACK {pk} do Agente {agent_s} para o Agente {agent_d} pelo socket");
                }
                else {
                    debug_println!("->-> Erro {{{e}}} ao enviar pacote {pk} do Agente {agent_s} para o Agente {agent_d}");}
                false
            }
        }
    }
}
