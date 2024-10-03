/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::{UdpSocket, SocketAddr};
use std::io::Error;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::collections::HashMap;

use crate::config::BUFFER_SIZE;
use super::packet::Packet;
// use super::failure_detection;

// Estrutura básica para a camada de comunicação por canais
pub struct Channel {
    socket: UdpSocket,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(bind_addr: SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_addr)?;
        Ok(Self { socket })
    }
    
    pub fn run(&self,
                tx_msgs: Sender<Packet>,
                send_rx: Receiver<(Sender<Packet>, SocketAddr, u32)>) {
        let skt = self.socket.try_clone().unwrap();
        thread::spawn(move || {
            Channel::listener(skt, tx_msgs, send_rx);
        });
    }

    fn listener(socket: UdpSocket,
                tx_msgs: Sender<Packet>,
                rx_acks: Receiver<(Sender<Packet>, SocketAddr, u32)>) {
        let mut sends: HashMap<SocketAddr, (Sender<Packet>, u32)> = HashMap::new();
        let mut messages_sequence_numbers: HashMap<SocketAddr, u32> = HashMap::new();
        loop {
            let mut buffer = [0; BUFFER_SIZE];
            let size;
            match socket.recv_from(&mut buffer) {   
                Ok((size_, _)) => { size = size_; },
                Err(e) => {
                    let agent = socket.local_addr().unwrap().port() % 100;
                    debug_println!("->-> Erro {{{e}}} no Agente {agent} ao receber pacote pelo socket");
                    continue
                },
            }
            // Simula perda de pacotes
            if rand::random::<f32>() < crate::config::LOSS_RATE {
                continue;
            }
            let packet: Packet = Packet::from_bytes(buffer, size);

            // Verifica se o pacote recebido é válido
            if !Channel::validate_message(&packet) {
                continue;
            }
            
            if packet.is_ack() {
                // Verifica se há alguém esperando pelo ACK recebido
                while let Ok((tx, key, start_seq)) = rx_acks.try_recv() {
                    sends.entry(key).or_insert((tx, start_seq));
                }
                // Se houver, encaminha o ACK para o remetente
                // Senão, ignora o ACK
                match sends.get_mut(&packet.header.src_addr) {
                    // Encaminha o ACK para a sender correspondente
                    Some(tupla) => {
                        let (tx, seq_num) = tupla;
                        if packet.header.seq_num < *seq_num {
                            continue;
                        }
                        *seq_num = packet.header.seq_num + 1;
                        let is_last = packet.is_last();
                        let src = packet.header.src_addr;
                        Channel::deliver(tx, packet);
                        if is_last {
                            sends.remove(&src);
                        }
                    }
                    None => continue,
                }
            } else {
                let next_seq_num = *messages_sequence_numbers.get(&packet.header.src_addr).unwrap_or(&0);
                if packet.header.seq_num > next_seq_num {
                    continue;
                }
                // Send ACK
                let ack = packet.header.get_ack();
                match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                    Ok(_) => (),
                    Err(e) => {
                        let agent_s = ack.src_addr.port() % 100;
                        let agent_d = ack.dst_addr.port() % 100;
                        let pk = ack.seq_num;
                        debug_println!("->-> Erro {{{e}}} ao enviar ACK {pk} do Agente {agent_s} para o Agente {agent_d} pelo socket");
                        continue
                    },
                }
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

    fn deliver(tx: &Sender<Packet>, packet: Packet) {
        let agent_s = packet.header.src_addr.port() % 100;
        let agent_d = packet.header.dst_addr.port() % 100;
        let pk = packet.header.seq_num;
        let is_ack = packet.is_ack();
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

    pub fn send(&self, packet: Packet) { 
        match self.socket.send_to(&packet.to_bytes(), packet.header.dst_addr) {
            Ok(_) => (),
            Err(e) => {
                let agent_s = packet.header.src_addr.port() % 100;
                let agent_d = packet.header.dst_addr.port() % 100;
                let pk = packet.header.seq_num;
                debug_println!("->-> Erro {{{e}}} ao enviar pacote {pk} do Agente {agent_s} para o Agente {agent_d}");
            }
        }
    }
}
