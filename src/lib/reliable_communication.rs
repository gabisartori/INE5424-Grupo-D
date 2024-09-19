/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use super::header::Header;
use crate::config::{BUFFER_SIZE, Node, TIMEOUT, W_SIZE};

use std::net::SocketAddr;
use std::sync::mpsc::{self, Sender};

pub struct ReliableCommunication {
    pub channel: Channel,
    pub host: SocketAddr,
    pub group: Vec<Node>,
    pub send_tx: mpsc::Sender<(Sender<Header>, SocketAddr)>,
    pub receive_rx: mpsc::Receiver<Vec<u8>>,
}

// TODO: Fazer com que a inicialização seja de um grupo

impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: SocketAddr, group: Vec<Node>) -> Self {
        let (send_tx, send_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();
        let channel = Channel::new(&host, send_rx, receive_tx).expect("\nFalha ao inicializar o canal no nível Rel_Com\n");
        
        Self { channel, host, group, send_tx, receive_rx}
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: Vec<u8>) {
        let mut base: usize = 0;
        let mut next_seq_num = 0;
        let (ack_tx, ack_rx) = mpsc::channel();
                let packages: Vec<&[u8]> = message.chunks(BUFFER_SIZE).collect();
        loop {
            if next_seq_num < base + W_SIZE && next_seq_num < packages.len() {
                // let mut pck: [u8] = packages[next_seq_num];
                let mut msg: Vec<u8> = packages[next_seq_num].to_vec();
                // enviando pacotes da janela
                let header = Header {
                    src_addr: self.host,
                    dst_addr: *dst_addr,
                    ack_num: 0,
                    seq_num: next_seq_num as u32,
                    msg_size: msg.len(),
                    checksum: 0,
                    flags: 0,
                    is_last: next_seq_num + 1 == packages.len(),
                    id: 0,
                    msg: msg,
                };
                self.send_tx.send((ack_tx.clone(), *dst_addr)).expect("Falha ao enviar mensagem\n");
                next_seq_num += 1;
            } else {
                // Lógica para confirmar recebimento de ACKs
                let mut timer = std::time::Instant::now();
                loop {
                    // enquanto não houver timeout nem não receber todos os ACKs
                    if timer.elapsed().as_millis() < TIMEOUT {
                        let header = ack_rx.recv().unwrap();
                        if header.ack_num == base as u32 {
                            base += 1;
                        } else {
                            // perdeu um pacote, reinicia a janela a partir do perdido
                            next_seq_num = base;                        
                            break;
                        }
                        if base == next_seq_num {
                            // todos os pacotes foram enviados
                            break;
                        }
                    } else {
                        // timeout estourado
                        next_seq_num = base;
                        break;
                    }
                }
                // se todos os pacotes foram enviados
                if next_seq_num >= packages.len() {
                    break;
                }
            }
        }
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut Vec<u8>) -> (usize, SocketAddr) {
        *buffer = self.receive_rx.recv().expect("Thread listener terminou e não há mais mensagens para receber\n");
        (buffer.len(), self.host)
    }
}

