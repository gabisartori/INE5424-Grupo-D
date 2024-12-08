/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver, RecvTimeoutError};
use std::time::Duration;

use logger::{log::SharedLogger, debug};
use crate::config::{BROADCAST, MESSAGE_TIMEOUT, BROADCAST_TIMEOUT};
use crate::channels::Channel;
use crate::failure_detection::FailureDetection;
use crate::node::Node;
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
    reg_to_snd_tx: Sender<SendRequest>,
}

impl RecAux for ReliableCommunication {}

impl ReliableCommunication {
    /// Starts a new thread to listen for any incoming messages
    /// This thread will be responsible for handling the destination of each received packet
    pub fn new(
        host: Node,
        group: Vec<Node>,
        logger: SharedLogger,
    ) -> Result<Arc<Self>, std::io::Error> {
        let channel = Channel::new(host.addr, logger.clone(), host.clone())?;
        let broadcast: Broadcast = match BROADCAST {
            "BEB" => Broadcast::BEB,
            "URB" => Broadcast::URB,
            "AB" => Broadcast::AB,
            _ => panic!("Falha ao converter broadcast {BROADCAST} para Broadcast"),
        };
        let group = Arc::new(Mutex::new(group));

        let (reg_to_snd_tx, reg_to_send_rx) = mpsc::channel();
        let (messages_tx, receive_rx) = mpsc::channel();
        let (reg_snd_to_listener_tx, reg_snd_rx) = mpsc::channel();
        let (reg_brd_to_listener_tx, reg_brd_rx) = mpsc::channel();
        let (broadcast_waiters_tx, brd_waiters_rx) = mpsc::channel();
        let (snd_acks_tx, snd_acks_rx) = mpsc::channel();
        let (brd_acks_tx, brd_acks_rx) = mpsc::channel();
        let (hb_tx, hb_rx) = mpsc::channel();

        let sender = RecSender::new(host.clone(), group.clone(),
            channel.clone(), reg_to_snd_tx.clone(), broadcast.clone(), logger.clone());

        let listener = RecListener::new(host.clone(), 
            group.clone(), channel.clone(), broadcast.clone(), logger.clone(),
            reg_to_snd_tx.clone());

        let mut failure_detection = FailureDetection::new(group.clone());

        // spawn failure detection thread
        let (heart_beats, agent_num) = FailureDetection::get_hbs(&group, &host);
        thread::spawn(move || {
            failure_detection.run(hb_rx, channel, heart_beats, agent_num);
        });

        // Spawn sender thread
        thread::spawn(move || {
            sender.run(
                snd_acks_rx, brd_acks_rx, reg_to_send_rx,
                reg_snd_to_listener_tx, reg_brd_to_listener_tx
            );
        });

        // Spawn listener thread
        thread::spawn(move || {
            listener.run(
                messages_tx, snd_acks_tx, brd_acks_tx,
                reg_snd_rx, reg_brd_rx, hb_tx, brd_waiters_rx,
            );
        });

        let message_timeout = MESSAGE_TIMEOUT;
        let broadcast_timeout = BROADCAST_TIMEOUT;
        let receive_rx = Mutex::new(receive_rx);

        Ok(Arc::new(Self {
            host,
            group,
            logger,
            broadcast,
            message_timeout,
            broadcast_timeout,
            broadcast_waiters_tx,
            receive_rx,
            reg_to_snd_tx,
        }))
    }

    /// Send a message to a specific destination
    pub fn send(&self, id: usize, message: Vec<u8>) -> u32 {
        let node = {
            let g = self.group.lock().expect("Erro ao enviar mensagem: Mutex lock do grupo falhou");
            g.get(id).cloned()
        };
        match node {
            Some(node) => {
                match Self::send_nonblocking(&self.reg_to_snd_tx, &node.addr, message).recv() {
                    Ok(result) => result,
                    Err(e) => {
                        debug!("Erro ao enviar mensagem na send: {e}");
                        0
                    }
                }
            },
            None => {
                debug!("Erro ao enviar mensagem: ID de destino não encontrado");
                0
            }
        }
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
            Err(RecvTimeoutError::Timeout) => {
                debug!("Timed out waiting for message");
                false
            },
            Err(RecvTimeoutError::Disconnected) => {
                debug!("Erro ao receber mensagem: Canal de comunicação desconectado");
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

    /// Register to receive broadcasts confirmations
    fn reg_to_brd(&self) -> Receiver<Vec<u8>> {
        let (broadcast_tx, broadcast_rx) = mpsc::channel::<Vec<u8>>();
        match self.broadcast_waiters_tx.send(broadcast_tx) {
            Ok(_) => {}
            Err(e) => {
                debug!("Erro ao registrar broadcast waiter: {e}");
            }
        }
        broadcast_rx
    }

    /// Listen for any broadcasts until your message arrives
    /// While there are broadcasts arriving, it means the leader is still alive
    /// If the channel times out before your message arrives, it means the leader died
    fn wait_for_brd(&self, broadcast_rx: &Receiver<Vec<u8>>, message: Vec<u8>) -> Result<u32, RecvTimeoutError> {
        loop {
            match broadcast_rx.recv_timeout(self.broadcast_timeout) {
                Ok(msg) => {
                    if msg == message {
                        let len = Self::get_livings(&self.group).len() as u32;
                        return Ok(len);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    return Err(RecvTimeoutError::Timeout);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    /// Best-Effort Broadcast: attempts to send a message to all nodes in the group and return how many were successful
    /// This algorithm does not garantee delivery to all nodes if the sender fails
    fn beb(&self, message: Vec<u8>) -> u32 {
        let result_rx = Self::brd_req(&self.reg_to_snd_tx, message);

        match result_rx.recv() {
            Ok(result) => result,
            Err(e) => {
                debug!("Erro ao receber resultado do BEB: {e}");
                0
            }
        }
    }

    /// Uniform Reliable Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all nodes receive the message if the sender does not fail
    fn urb(&self, message: Vec<u8>) -> u32 {
        let rx = self.reg_to_brd();
        Self::brd_req(&self.reg_to_snd_tx, message.clone());
        match self.wait_for_brd(&rx, message) {
            Ok(result) => {
                result
            },
            Err(_) => {
                0
            }
        }
    }

    /// Atomic Broadcast: sends a message to all nodes in the group and returns how many were successful
    /// This algorithm garantees that all messages are delivered in the same order to all nodes
    fn ab(&self, message: Vec<u8>) -> u32 {
        let broadcast_rx = self.reg_to_brd();
        // Constantly try to get a leader and ask it to broadcast
        let mut prev_leader = self.host.agent_number;
        loop {
            let leader = Self::get_leader(&self.group, &self.host).agent_number;
            if leader == self.host.agent_number {
                // Start the broadcast
                debug!("Sou o líder, começando o broadcast");
                Self::brd_req(&self.reg_to_snd_tx, message.clone());
            } else if leader != prev_leader {
                // Ask the leader to broadcast and wait for confirmation
                let (request, request_result_rx) = SendRequest::new (
                    message.clone(),
                    SendRequestData::RequestLeader {},
                );
                match self.reg_to_snd_tx.send(request) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("Erro ao enviar request de AB: {e}");
                    }
                }
                prev_leader = leader;
                // Wait for the request result
                match request_result_rx.recv() {
                    Ok(0) => {
                        // debug!("Falha em enviar mensagem para o líder, tentando novamente...");
                        // continue;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        debug!("Erro ao fazer requisitar AB para o lider: {e}");
                        continue;
                    }
                }
            } else {
                // never make more than 1 request to the same leader
                debug!("Tentando de novo o líder {leader} para fazer broadcast");
            }
            match self.wait_for_brd(&broadcast_rx, message.clone()) {
                Ok(result) => {
                    debug!("Recebeu a mensagem de broadcast de volta");
                    return result;
                },
                Err(RecvTimeoutError::Timeout) => {
                    debug!("Timed out ao esperar por broadcast do Agente {leader}");
                    continue;
                }
                Err(e) => {
                    debug!("Erro ao esperar por Atomic Broadcast: {e}");
                    panic!("{:?}", e)
                }
            }
        }
    }
}
