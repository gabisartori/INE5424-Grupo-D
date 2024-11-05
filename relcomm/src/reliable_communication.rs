/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

use std::clone::Clone;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver, RecvTimeoutError};
use std::time::Duration;

use crate::channels::Channel;
use crate::config::{BROADCAST, TIMEOUT, TIMEOUT_LIMIT, MESSAGE_TIMEOUT, BROADCAST_TIMEOUT, GOSSIP_RATE, W_SIZE};
use crate::node::Node;
use logger::debug_println;
use logger::log::SharedLogger;
use crate::rec_aux::{SendRequest, SendRequestData, Broadcast, RecAux};
use crate::rec_listener::RecListener;
use crate::rec_sender::RecSender;

pub struct ReliableCommunication {
    pub host: Node,
    pub group: Arc<Mutex<Vec<Node>>>,
    pub logger: SharedLogger,
    broadcast: Broadcast,
    message_timeout: Duration,
    broadcast_timeout: Duration,
    broadcast_waiters_tx: Sender<Sender<Vec<u8>>>,
    receive_rx: Mutex<Receiver<Vec<u8>>>,
    register_to_sender_tx: Sender<SendRequest>,
}

impl RecAux for ReliableCommunication {}

impl ReliableCommunication {
    /// Starts a new thread to listen for any incoming messages
    /// This thread will be responsible for handling the destination of each received packet
    pub fn new(
        host: Node,
        group: Vec<Node>,
        logger: SharedLogger,
    ) -> Result<Self, std::io::Error> {
        let channel = Channel::new(host.addr)?;
        let broadcast: Broadcast = match BROADCAST {
            "BEB" => Broadcast::BEB,
            "URB" => Broadcast::URB,
            "AB" => Broadcast::AB,
            _ => panic!("Falha ao converter broadcast {BROADCAST} para Broadcast"),
        };
        let timeout_limit: u32 = TIMEOUT_LIMIT;
        let gossip_rate: usize = GOSSIP_RATE;
        let w_size: usize = W_SIZE;

        let timeout = Duration::from_micros(TIMEOUT);
        let message_timeout = Duration::from_millis(MESSAGE_TIMEOUT);
        let broadcast_timeout = Duration::from_millis(BROADCAST_TIMEOUT);
        let group = Arc::new(Mutex::new(group));
        let dst_seq_num_cnt = Mutex::new(HashMap::new());

        let (register_to_sender_tx, register_to_sender_rx) = mpsc::channel();
        let (receive_tx, receive_rx) = mpsc::channel();

        let (register_to_listener_tx, register_to_listener_rx) = mpsc::channel();
        let (broadcast_waiters_tx, broadcast_waiters_rx) = mpsc::channel();
        let receive_rx = Mutex::new(receive_rx);

        let (acks_tx, acks_rx) = mpsc::channel();

        let sender = RecSender::new(host.clone(), dst_seq_num_cnt, channel.clone(), Arc::clone(&group), broadcast.clone(), logger.clone(), timeout, timeout_limit, w_size, gossip_rate);
        let listener = RecListener::new(host.clone(), Arc::clone(&group), channel, broadcast.clone(), logger.clone(), register_to_sender_tx.clone());

        // Spawn all threads
        std::thread::spawn(move || {
            sender.run(acks_rx, register_to_sender_rx, register_to_listener_tx);
        });
        std::thread::spawn(move || {
            listener.run(
                receive_tx,
                acks_tx,
                register_to_listener_rx,
                broadcast_waiters_rx,
            );
        });
        Ok(Self {
            host,
            group,
            logger,
            broadcast,
            message_timeout,
            broadcast_timeout,
            broadcast_waiters_tx,
            receive_rx,
            register_to_sender_tx,
        })
    }

    /// Send a message to a specific destination
    pub fn send(&self, id: usize, message: Vec<u8>) -> u32 {
        let node = {
            let g = self.group.lock().expect("Erro ao enviar mensagem: Mutex lock do grupo falhou");
            g.get(id).cloned()
        };
        match node {
            Some(node) => {
                match self.send_nonblocking(&node.addr, message).recv() {
                    Ok(result) => result,
                    Err(e) => {
                        debug_println!("Erro ao enviar mensagem na send: {e}");
                        0
                    }
                }
            },
            None => {
                debug_println!("Erro ao enviar mensagem: ID de destino não encontrado");
                0
            }
        }
    }

    fn send_nonblocking(&self, dst_addr: &SocketAddr, message: Vec<u8>) -> Receiver<u32> {
        let (request, result_rx) = SendRequest::new(
            message,
            SendRequestData::Send {
                destination_address: *dst_addr,
            },
        );
        match self.register_to_sender_tx.send(request) {
            Ok(_) => {}
            Err(e) => {
                debug_println!("Erro ao registrar request: {e}");
            }
        }
        result_rx
    }

    /// Read one already received message or wait for a message to arrive
    pub fn receive(&self, buffer: &mut Vec<u8>) -> bool {
        match self
            .receive_rx
            .lock()
            .expect("Erro ao receber mensagem: Mutex lock o receive_rx falhou")
            .recv_timeout(self.message_timeout)
        {
            Ok(msg) => {
                buffer.extend(msg);
                true
            }
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => {
                debug_println!("Erro ao receber mensagem: Canal de comunicação desconectado");
                false
            }
        }
    }

    /// Broadcasts a message, reliability level may be configured in the config file
    pub fn broadcast(&self, message: Vec<u8>) -> u32 {
        match self.broadcast {
            Broadcast::BEB => self.beb(message),
            Broadcast::URB => self.urb(message),
            Broadcast::AB => self.ab(message),
        }
    }

    /// Best-Effort Broadcast: attempts to send a message to all nodes in the group and return how many were successful
    /// This algorithm does not garantee delivery to all nodes if the sender fails
    fn beb(&self, message: Vec<u8>) -> u32 {
        let result_rx = Self::brd_req(&self.register_to_sender_tx, message);

        match result_rx.recv() {
            Ok(result) => result,
            Err(e) => {
                debug_println!("Erro ao receber resultado do BEB: {e}");
                0
            }
        }
    }

    /// Uniform Reliable Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all nodes receive the message if the sender does not fail
    fn urb(&self, message: Vec<u8>) -> u32 {
        Self::brd_req(&self.register_to_sender_tx, message);
        // TODO: Make it so that URB waits for at least one node to receive the message
        self.group.lock()
        .expect("Falha ao fazer o URB, Não obteve lock de Grupo").len() as u32
    }

    /// Atomic Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all messages are delivered in the same order to all nodes
    fn ab(&self, message: Vec<u8>) -> u32 {
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<Vec<u8>>();
        match self.broadcast_waiters_tx.send(broadcast_tx) {
            Ok(_) => {}
            Err(e) => {
                debug_println!("Erro ao registrar broadcast waiter no AB: {e}");
            }
        }
        
        // Constantly try to get a leader and ask it to broadcast
        loop {
            let leader = Self::get_leader(&self.group, &self.host);
            if leader == self.host {
                // Start the broadcast
                Self::brd_req(&self.register_to_sender_tx, message.clone());
                return self.group.lock().expect("Erro ao terminar o AB, não obteve-se o Mutex lock do grupo").len() as u32;
            }
            // Ask the leader to broadcast and wait for confirmation
            let (request, request_result_rx) = SendRequest::new (
                message.clone(),
                SendRequestData::RequestLeader {},
            );
            match self.register_to_sender_tx.send(request) {
                Ok(_) => {}
                Err(e) => {
                    debug_println!("Erro ao enviar request de AB: {e}");
                }
            }

            // Wait for the request result
            match request_result_rx.recv() {
                Ok(0) => {
                    debug_println!("Agente {} falhou em enviar mensagem para o líder, tentando novamente...", self.host.agent_number);
                    continue;
                }
                Ok(_) => {}
                Err(e) => {
                    debug_println!("Erro ao fazer requisitar AB para o lider: {e}");
                    continue;
                }
            }

            // Listen for any broadcasts until your message arrives
            // While there are broadcasts arriving, it means the leader is still alive
            // If the channel times out before your message arrives, it means the leader died
            loop {
                let msg = broadcast_rx.recv_timeout(self.broadcast_timeout);
                match msg {
                    Ok(msg) => {
                        if msg == message {
                            debug_println!("Agente {} recebeu a mensagem de broadcast de volta", self.host.agent_number);
                            return self.group.lock().expect("Erro ao terminar o AB, não obteve-se o Mutex lock do grupo").len() as u32;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        debug_println!("Agent {} timed out when waiting broadcast from leader, will try again", self.host.agent_number);
                        break;
                    }
                    Err(e) => {
                        panic!("{:?}", e)
                    }
                }
            }
        }
    }
}
