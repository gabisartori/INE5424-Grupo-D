/*
A camada de comunicação mais baixa, representa os canais de comunicação (channels)
e implementa sockets para comunicação entre os processos participantes.
*/
use std::net::{UdpSocket, SocketAddr};
use std::io::Error;
use std::sync::mpsc;
use std::thread;
use std::collections::{HashMap, VecDeque};

use crate::config::BUFFER_SIZE;
use super::packet::Packet;
// use super::failure_detection;

// Estrutura básica para a camada de comunicação por canais
pub struct Channel {
    socket: UdpSocket,
}

impl Channel {
    // Função para criar um novo canal
    pub fn new(
        bind_addr: SocketAddr,
        send_rx: mpsc::Receiver<(mpsc::Sender<Packet>, SocketAddr)>,
        receive_tx: mpsc::Sender<Packet>
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
        thread::spawn(move || Channel::listener(skt, send_rx, receive_tx));
        Ok(Self { socket })
    }
    
    fn listener(socket: UdpSocket, rx_acks: mpsc::Receiver<(mpsc::Sender<Packet>, SocketAddr)>, tx_msgs: mpsc::Sender<Packet>) {
        let mut sends: HashMap<SocketAddr, mpsc::Sender<Packet>> = HashMap::new();
        let mut msgs: HashMap<SocketAddr, VecDeque<Packet>> = HashMap::new();
        loop {
            let mut buffer = [0; BUFFER_SIZE];
            let size;
            match socket.recv_from(&mut buffer) {
                Ok((size_, _)) => { size = size_; },
                Err(_) => {
                    let agent = socket.local_addr().unwrap().port() % 100;
                    debug_println!("Erro no Agente {agent} ao receber pacote pelo socket");
                    continue
                },
            }
            let packet: Packet = Packet::from_bytes(buffer, size);

            // Verifica se o pacote recebido é válido
            let next_seq_num = match msgs.get(&packet.header.src_addr) {
                Some(msg) => {
                    msg.back().map_or(0, |last| last.header.seq_num + 1)
                },
                None => {
                    msgs.insert(packet.header.src_addr, VecDeque::new());
                    0
                }
            };
            if !Channel::validate_message(&packet, next_seq_num) {
                continue;
            }
            
            if packet.is_ack() {
                // Verifica se há alguém esperando pelo ACK recebido
                // Se houver, encaminha o ACK para o remetente
                // Senão, ignora o ACK
                Channel::get_txs(&rx_acks, &mut sends);
                match sends.get(&packet.header.src_addr) {
                    // Encaminha o ACK para a sender correspondente
                    Some(tx) => {
                        match tx.send(packet.clone()) {
                            Ok(_) => {
                                if packet.is_last() {
                                    sends.remove(&packet.header.src_addr);
                                }
                            },
                            Err(_) => {
                                let agent_s = packet.header.src_addr.port() % 100;
                                let agent_d = packet.header.dst_addr.port() % 100;
                                debug_println!("Erro ao encaminhar ACK vindo do Agente {agent_s} para o Agente {agent_d}");
                                continue;                             
                            },
                        }
                    }
                    None => continue,
                }
            } else {
                // Send ACK
                let ack = packet.header.get_ack();
                match socket.send_to(&ack.to_bytes(), ack.dst_addr) {
                    Ok(_) => (),
                    Err(_) => {
                        let agent = ack.dst_addr.port() % 100;
                        debug_println!("Erro ao enviar ACK para o Agente {agent} pelo socket");
                        continue
                    },
                }
                // Encaminhar o pacote para a fila de mensagens
                msgs.get_mut(&packet.header.src_addr).unwrap().push_back(packet.clone());
                let msg = msgs.entry(packet.header.src_addr).or_insert(VecDeque::new());
                // TODO: Refatorar para evitar repetição de código
                if packet.is_last() {
                    if msg[0].is_last() {
                        msg.pop_front();
                    }
                    while let Some(pckg) = msg.pop_front() {
                        let last = pckg.is_last();
                        match tx_msgs.send(pckg.clone()) {
                            Ok(_) => (),
                            Err(_) => {
                                let agent = pckg.header.dst_addr.port() % 100;
                                debug_println!("Erro ao entregar o pacote para o Agente {agent}");
                                msg.push_front(pckg.clone());
                                break;
                            }
                        }
                        if last { break; }
                    }
                    msg.push_back(packet.clone());
                }
            }
        }
    }

    fn get_txs(rx: &mpsc::Receiver<(mpsc::Sender<Packet>, SocketAddr)>, map: &mut HashMap<SocketAddr, mpsc::Sender<Packet>>) {
        while let Ok((tx, key)) = rx.try_recv() {
            map.entry(key).or_insert(tx);
        }
    }

    fn validate_message(packet: &Packet, next_seq_num:u32 ) -> bool {
        // Checksum
        let c1: bool = packet.header.checksum == Packet::checksum(&packet.header, &packet.data);
        // Se o pacote é um ACK, o número de sequência não precisa ser verificado
        let c2: bool = packet.header.seq_num == next_seq_num || packet.is_ack();
        c1 && c2
    }

    pub fn send(&self, packet: Packet) { 
        match self.socket.send_to(&packet.to_bytes(), packet.header.dst_addr) {
            Ok(_) => (),
            Err(_) => {
                let agent = packet.header.dst_addr.port() % 100;
                debug_println!("Erro ao enviar pacote para o Agente {agent}");
            }
        }
    }
}
