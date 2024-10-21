/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::cell::RefCell;
use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, mpsc::{Sender, Receiver}};
use std::thread;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::config::{BUFFER_SIZE, LOSS_RATE};
use super::packet::Packet;
// use super::failure_detection;

// Estrutura básica para a camada de comunicação por canais
pub struct Channel {
    socket: Arc<UdpSocket>,
    packets_tx: Sender<Packet>,
    acks_tx: Sender<Packet>,
    register_from_sender_rx: Receiver<(SocketAddr, u32)>,
    expected_acks: RefCell<HashMap<SocketAddr, u32>>,
    expected_messages: RefCell<HashMap<SocketAddr, u32>>,
}

impl Channel {
    /// Constructor
    pub fn new(socket: Arc<UdpSocket>, packets_tx: Sender<Packet>, acks_tx: Sender<Packet>, register_from_sender_rx: Receiver<(SocketAddr, u32)>) -> Self {
        Self { socket, packets_tx, acks_tx, register_from_sender_rx, expected_acks: RefCell::new(HashMap::new()), expected_messages: RefCell::new(HashMap::new()) }
    }
    
    /// Spawns the thread that listens for incoming messages
    pub fn run(self) {
        thread::spawn(move || {
            self.listener();
        });
    }

    /// Listener for incoming messages
    /// This function will be responsible for routing and filtering the received packets to the correct destination
    /// Routing is done using the channels stablished by those who are waiting for something
    /// Filtering is done by checking the checksum and sequence number of the received packet
    fn listener(&self) {
        loop {
            let packet;
            match self.receive() {
                Some(p) => packet = p,
                None => continue 
            };
            // Verifica se o pacote recebido é válido
            if !self.validate_message(&packet) {continue;}
            
            if packet.header.is_ack() {
                self.process_acks(packet);
            } else {
                // Verificar se o pacote é o próximo esperado
                let next_seq_num = *self.expected_messages.borrow().get(&packet.header.src_addr).unwrap_or(&0);
                debug_println!("Agent {} received packet {}, expected {}", packet.header.dst_addr.port(), packet.header.seq_num, next_seq_num);
                if packet.header.seq_num > next_seq_num {continue;}
                // Send ACK
                if !self.send(&packet.get_ack()) {continue;}
                // Encaminhar o pacote para a fila de mensagens se for o próximo esperado
                if packet.header.seq_num < next_seq_num { continue; }
                
                self.expected_messages.borrow_mut().insert(packet.header.src_addr, packet.header.seq_num + 1);
                Channel::deliver(&self.packets_tx, packet);
            }
        }
    }

    /// Validates the received message
    /// For now, only validates the checksum
    fn validate_message(&self, packet: &Packet) -> bool {
        // Checksum
        let c1: bool = packet.header.checksum == Packet::checksum(&packet.header, &packet.data);
        c1
    }

    /// Checks for senders waiting for the ACK received
    /// If there is someone waiting, forwards the ACK to the sender
    fn process_acks(&self, packet: Packet) {
        debug_println!("Agent {} received ack {}", packet.header.dst_addr.port(), packet.header.seq_num);
        // Verifica se há alguém esperando pelo ACK recebido
        while let Ok((key, start_seq)) = self.register_from_sender_rx.try_recv() {
            match self.expected_acks.borrow_mut().entry(key) {
                Entry::Occupied(mut entry) => {
                    let seq_num = entry.get_mut();
                    *seq_num = start_seq;
                },
                Entry::Vacant(entry) => {
                    entry.insert(start_seq);
                }
            }
        }
        // Encaminha o ACK para o destinatário se houver um
        match self.expected_acks.borrow_mut().get_mut(&packet.header.src_addr){
            Some(seq_num) => {
                if packet.header.seq_num < *seq_num { return; }
                *seq_num = packet.header.seq_num + 1;
                Channel::deliver(&self.acks_tx, packet);
            }
            None => {
                debug_println!("->-> ACK recebido sem destinatário esperando");
            },
        }
    }

    /// Reads a packet from the socket or waits for a packet to arrive
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

    /// Wrapper for Sender::send, with some print statements
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

    /// Wrapper for UdpSocket::send_to, with some print statements
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
