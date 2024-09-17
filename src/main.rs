#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::ExitCode;
use std::thread;
use std::sync::Arc;
use std::env;
use rand::Rng;
// use std::io::Write;
// use std::fs::{File, OpenOptions};

mod lib {
    pub mod reliable_communication;
    pub mod channels;
    pub mod failure_detection;
}
use lib::reliable_communication::ReliableCommunication;

// Importa as configurações de endereços dos processos
mod config;
use config::{Node, BUFFER_SIZE, LOCALHOST, NODES};

struct Agent {
    id: u32,
    addr: SocketAddr,
    communication: ReliableCommunication
}

impl Agent {
    fn new(id: u32, addr: SocketAddr, nodes: Vec<Node>) -> Self {
        Agent {
            id,
            addr,
            communication: ReliableCommunication::new(addr, nodes)
        }
    }

    fn listener(&self) {
        // println!("Agent {} is listening", self.id);
        // let path = format!("testes/listener_{}.txt", self.id);
        // let mut file: File = OpenOptions::new().append(true).open(path).unwrap();
        // loop
        {
            let mut message: Vec<u8> = Vec::new();
            let (size, sender) = self.communication.receive(&mut message);
            // divide a mensagem em chunks de forma que possa ser convertida para string
            let chuncked_msg = message.chunks(BUFFER_SIZE);
            let mut full_msg: Vec<&str> = Vec::new();
            for m in chuncked_msg {
                let pck = std::str::from_utf8(&m).unwrap();
                full_msg.push(pck);
            }
            let msg = full_msg.join("");
            let msf = format!("Agent {} receiving {} bytes from {}\n--> Message:\n{}", self.id, size, sender, msg);
            // write message to a listener.txt file
            // file.write_all(msf.as_bytes()).unwrap();
            println!("{}", msf);
        }
    }

    fn sender(&self, user_controlled: bool) {
        // println!("Agent {} is sending messages", self.id);

        // Choice of destination for each message the agent sends
        let mut destination: u32;
        // loop
        {
            // Pick a random node to send a message to
            // If the random node is the agent itself, pick another one
            loop {
                destination = rand::thread_rng().gen_range(0..self.communication.group.len() as u32);
                if destination != self.id { break; }
            }

            // Send message to the selected node
            /*
                TODO: Isso aqui tá uma gambiarra eu não entendo por que fazer direto
                format!("Hello from agent {}", self.id).as_bytes()
                Dá erro.
                =>=> Porque format! retorna um String e não um &str,
                => se não for alocado na memória (em uma variável), não dá pra passar a referência (&[u8])
            */
            // let msg: String = format!("Hello from agent {}", self.id);
            let msg: String = config::LARGE_MSG.to_string();
            let msg: Vec<u8> = msg.as_bytes().to_vec();
            println!("Agent {} sending message to agent {}", self.id, destination);
            self.communication.send(&(self.communication.group[destination as usize].addr), msg);

            // Sleep for a random amount of time
            // thread::sleep(std::time::Duration::from_secs(rand::thread_rng().gen_range(1..10)));
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
}


fn main() {
    let mut nodes: Vec<Node> = Vec::new();
    let mut local_agents: Vec<thread::JoinHandle<()>> = Vec::new();

    // Contruir vetor unificando os nós locais e os remotos
    for i in 0..config::AGENT_NUM {
        nodes.push(Node{addr: SocketAddr::new(config::LOCALHOST, 3100 + (i as u16)), agent_number: i});
    }

    if let Some(remote_nodes) = config::NODES {
        for node in remote_nodes {
            nodes.push(Node{addr: node.addr, agent_number: node.agent_number});
        }
    }

    // Inicializar os agentes locais
    for i in 0..config::AGENT_NUM {
        let agent: Arc<Agent> = Arc::new(Agent::new(i, SocketAddr::new(config::LOCALHOST, 3100 + i as u16), nodes.clone()));
        let agent_handler = thread::spawn(move || agent.run());
        local_agents.push(agent_handler);
    }

    for agent in local_agents {
        agent.join().unwrap();
    }

}
