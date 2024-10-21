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
    messages_tx: Sender<Vec<u8>>,
    acks_tx: Sender<Packet>,
    register_from_sender_rx: Receiver<(SocketAddr, u32)>,
    expected_acks: RefCell<HashMap<SocketAddr, u32>>,
    packets_per_source: RefCell<HashMap<SocketAddr, Vec<Packet>>>,
}

impl Channel {
    /// Constructor
    pub fn new(socket: Arc<UdpSocket>, messages_tx: Sender<Vec<u8>>, acks_tx: Sender<Packet>, register_from_sender_rx: Receiver<(SocketAddr, u32)>) -> Self {
        Self { socket, messages_tx, acks_tx, register_from_sender_rx, expected_acks: RefCell::new(HashMap::new()), packets_per_source: RefCell::new(HashMap::new()) }
    }
    
    /// Spawns the thread that listens for incoming messages
    /// This thread will be responsible for routing and filtering the received packets to the correct destination
    /// Routing is done using the channels stablished by those who are waiting for something
    /// Filtering is done by checking the checksum and sequence number of the received packet
    pub fn run(self) {
        thread::spawn(move || {
            loop {
                let packet = self.receive();
                if self.validate_message(&packet) { self.handle_packet(packet); }
            }
        });
    }

    /// Validates the received message
    /// For now, only validates the checksum
    fn validate_message(&self, packet: &Packet) -> bool {
        // Checksum
        let c1: bool = packet.header.checksum == Packet::checksum(&packet.header, &packet.data);
        c1
    }

    fn handle_packet(&self, packet: Packet) {
        if packet.header.is_ack() {
            self.handle_ack(packet);
        } else {
            self.handle_message(packet);
        }
    }

    /// Checks for senders waiting for the ACK received
    /// If there is someone waiting, forwards the ACK to the sender
    fn handle_ack(&self, packet: Packet) {
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

    /// Handles received packets
    /// Sending ACKs and building messages
    fn handle_message(&self, packet: Packet) {
        let mut packets_per_source = self.packets_per_source.borrow_mut();
        let packets = packets_per_source.entry(packet.header.src_addr).or_insert(Vec::new());

        let expected = packets.last().map_or(0, |p| p.header.seq_num + 1);

        // Ignore the packet if the sequence number is higher than expected
        if packet.header.seq_num > expected { return; }
        // Send ack otherwise
        self.send(&packet.get_ack());
        if packet.header.seq_num < expected { return; }
        
        if packet.header.is_last() {
            let mut message = Vec::new();
            if !packets.is_empty() && packets.first().unwrap().header.is_last() { packets.remove(0); }
            for packet in packets.iter() {
                message.extend(&packet.data);
            }
            message.extend(&packet.data);
            self.messages_tx.send(message).unwrap();
            packets.clear();
        }
        packets.push(packet);
    }

    /// Reads a packet from the socket or waits for a packet to arrive
    fn receive(&self) -> Packet {
        loop {
            let mut buffer = [0; BUFFER_SIZE];
            let size;
            match self.socket.recv_from(&mut buffer) {
                Ok((size_, _)) => { size = size_; },
                Err(e) => {
                    let agent = self.socket.local_addr().unwrap().port() % 100;
                    debug_println!("->-> Erro {{{e}}} no Agente {agent} ao receber pacote pelo socket");
                    continue;
                },
            }
            // Simula perda de pacotes
            if rand::random::<f32>() < LOSS_RATE { continue; }
            return Packet::from_bytes(buffer, size);
        }
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
