/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::{UdpSocket, SocketAddr};
use std::io::Error;
use std::thread;
use std::collections::HashMap;

use crate::config::BUFFER_SIZE;
use super::packet::Packet;
use super::message_queue::MessageQueue;
// use super::failure_detection;

// Estrutura básica para a camada de comunicação por canais
pub struct Channel {
    socket: UdpSocket,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(
        bind_addr: SocketAddr,
        send_rx: MessageQueue<(MessageQueue<Packet>, SocketAddr, u32)>,
        receive_tx: MessageQueue<Packet>,
        ) -> Result<Self, Error> {
        
        let socket = UdpSocket::bind(bind_addr)?;
        let skt: UdpSocket = match socket.try_clone() {
            Ok(s) => s,
            Err(_) => {
                let agent = bind_addr.port() % 100;
                let erro = format!("Erro ao clonar o socket do Agente {agent}");
                return Err(Error::new(std::io::ErrorKind::Other, erro))
            },
        };
        thread::spawn(move || Channel::listener(skt, receive_tx, send_rx));
        Ok(Self { socket })
    }
    
    fn listener(socket: UdpSocket,
                tx_msgs: MessageQueue<Packet>,
                rx_acks: MessageQueue<(MessageQueue<Packet>, SocketAddr, u32)>) {
        let mut sends: HashMap<SocketAddr, (MessageQueue<Packet>, u32)> = HashMap::new();
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
                        if packet.header.seq_num < tupla.1 {
                            continue;
                        }
                        tupla.1 = packet.header.seq_num + 1;
                        let tx = &tupla.0;
                        match tx.send(packet.clone()) {
                            Ok(_) => {
                                if packet.is_last() {
                                    sends.remove(&packet.header.src_addr);
                                }
                            },
                            Err(e) => {
                                let agent_s = packet.header.src_addr.port() % 100;
                                let agent_d = packet.header.dst_addr.port() % 100;
                                let pk = packet.header.seq_num;
                                debug_println!("->-> Erro {{{e}}} ao enviar ACK {pk}, do Agente {agent_s} para o Agente {agent_d} pelo canal.");
                                sends.remove(&packet.header.src_addr);
                                continue;                      
                            },
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
                match tx_msgs.send(packet.clone()) {
                    Ok(_) => (),
                    Err(e) => {
                        let agent_s = packet.header.src_addr.port() % 100;
                        let agent_d = packet.header.dst_addr.port() % 100;
                        let pk = packet.header.seq_num;
                        debug_println!("->-> Erro {{{e}}} ao enviar pacote {pk} do Agente {agent_s} para o Agente {agent_d} pelo canal");
                        continue
                    },
                }
            }
        }
    }

    fn validate_message(packet: &Packet) -> bool {
        // Checksum
        let c1: bool = packet.header.checksum == Packet::checksum(&packet.header, &packet.data);
        c1
    }

    pub fn send(&self, packet: Packet) { 
        match self.socket.send_to(&packet.to_bytes(), packet.header.dst_addr) {
            Ok(_) => (),
            Err(_) => {
                let agent_s = packet.header.src_addr.port() % 100;
                let agent_d = packet.header.dst_addr.port() % 100;
                let pk = packet.header.seq_num;
                debug_println!("->-> Erro ao enviar pacote {pk} do Agente {agent_s} para o Agente {agent_d}");
            }
        }
    }
}
