#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

use std::net::SocketAddr;
use std::thread;
use std::sync::Arc;
use std::env;
use rand::Rng;

mod lib {
    pub mod reliable_communication;
    pub mod channels;
    pub mod failure_detection;
}
use lib::reliable_communication::ReliableCommunication;

// Importa as configurações de endereços dos processos
mod config;
use config::{Node, BUFFER_SIZE, FAILURE_DETECTION_INTERVAL, HEARTBEAT_INTERVAL, NODES, TIMEOUT};

struct Agent {
    addr: SocketAddr,
    agent_number: u32,
    communication: ReliableCommunication
}

impl Agent {
    fn new(addr: SocketAddr, agent_number: u32) -> Agent {
        Agent {
            addr,
            agent_number,
            communication: ReliableCommunication::new(addr, NODES.to_vec())
        }
    }

    fn listener(&self) {
        println!("Agent {} is listening", self.agent_number);
        loop {
            let mut message: [u8; 1024] = [0; 1024];
            let (size, sender) = self.communication.receive(&mut message);
            println!("\nAgent {} receiving {} bytes from {}\nMessage: {}\n",
                self.agent_number, size, sender, std::str::from_utf8(&message).unwrap());
        }
    }

    fn sender(&self, user_controlled: bool) {
        println!("Agent {} is sending messages", self.agent_number);

        for node in NODES.to_vec() {
            if node.addr == self.addr { continue; }
            println!("\nAgent {} sending message to Agent {}\n", self.agent_number, node.agent_number);
            
            let message: [u8; BUFFER_SIZE] = self.format_message(&"Hello from Agent".to_string(), self.agent_number);
            self.communication.send(&(node.addr), &message);
            thread::sleep(std::time::Duration::from_secs(rand::thread_rng().gen_range(1..10)));
        }
    }

    fn run(self: Arc<Self>) {
        let listener_clone = Arc::clone(&self);
        let sender_clone = Arc::clone(&self);

        let listener = thread::spawn(move || listener_clone.listener());
        let sender = thread::spawn(move || sender_clone.sender(false));

        listener.join().unwrap();
        sender.join().unwrap();
    }

    fn format_message(&self, message: &String, id: u32) -> [u8; BUFFER_SIZE] {
        let message = format!("{} {}", message, id);
        let mut message_array: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
        for i in 0..message.len() {
            message_array[i] = message.as_bytes()[i];
        }
        message_array
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();
    let mut agent_number = match args.len() {
        2 => args[1].parse::<u32>().unwrap(),
        _ => 4,
    };

    if agent_number > NODES.len() as u32 {
        agent_number = NODES.len() as u32;
    }

    let mut agent_handlers: Vec<thread::JoinHandle<()>> = Vec::new();

    for i in 0..agent_number {
        let node: Node  = NODES[i as usize].clone();
        let agent: Arc<Agent> = Arc::new(Agent::new(node.addr, i));
        agent_handlers.push(thread::spawn(move || agent.run()));
    }

    println!();

    for handler in agent_handlers {
        handler.join().unwrap();
    }
}
