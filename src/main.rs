#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::thread;
use std::sync::Arc;
use std::env;
use rand::Rng;

mod lib {
    pub mod reliable_communication;
    pub mod channels;
    pub mod failure_detection;
    pub mod header;
}
use lib::reliable_communication::ReliableCommunication;
use lib::header::HEADER_SIZE;

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
        // let mut file: std::fs::File;
        for _ in 0..3
        {
            let mut message: Vec<u8> = Vec::new();
            if config::DEBUG {
                println!("\n-------------\nAGENTE {} VAI RECEBER UMA MENSAGEM\n-------------", self.id);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            self.communication.receive(&mut message);
            let msg = String::from_utf8_lossy(&message);
            let msf = format!("Agent {} recieved Message:\n-->\n{}", self.id, msg);
            // write message to a listener.txt file
            if config::DEBUG {
                println!("\n-------------\nMENSAGEM RECEBIDA POR AGENTE {}\n-------------\n", self.id);
                let _ = std::io::Write::flush(&mut std::io::stdout());
                let path = format!("target/listener_{}.txt", self.id);
                let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                                    .create(true)
                                                    .append(true)
                                                    .open(path) {
                    Ok(f) => f,
                    Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
                };
                std::io::Write::write_all(&mut file, msf.as_bytes().trim_ascii_end())
                .expect("Erro ao escrever no arquivo");
            } else {
                println!("{}", msf);
            }
        }
    }

    fn sender(&self) {
        let mut destination: u32 = (self.id + 1) % self.communication.group.len() as u32;
        for _ in 0..3
        {
            // Pick a random node to send a message to
            // loop {
            //     destination = rand::thread_rng().gen_range(0..self.communication.group.len() as u32);
            //     if destination != self.id { break; }
            // }

            // Send message to the selected node
            // let msg: String = format!("Hello from agent {}", self.id);
            let msg: String = config::LARGE_MSG.to_string();
            // let msg: String = format!("Hello");
            let msg: Vec<u8> = msg.as_bytes().to_vec();
            if config::DEBUG {
                println!("\n-------------\nAGENTE {} VAI ENVIAR A MENSAGEM PARA AGENTE {}\n-------------\n",
                self.id, destination);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            self.communication.send(&(self.communication.group[destination as usize].addr), msg);
            if config::DEBUG {
                println!("\n-------------\nAGENTE {} ENVIOU A MENSAGEM PARA AGENTE {}\n-------------",
                self.id, destination);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            // Sleep for a random amount of time
            // thread::sleep(std::time::Duration::from_secs(rand::thread_rng().gen_range(1..10)));
        }
    }

    pub fn run(self: Arc<Self>) {
        let sender_clone = Arc::clone(&self);
        let sender = thread::spawn(move || sender_clone.sender());
        let listener_clone = Arc::clone(&self);
        let listener = thread::spawn(move || listener_clone.listener());
        sender.join().unwrap();
        listener.join().unwrap();
}
}


fn main() {
    assert!(config::AGENT_NUM > 0, "Número de agentes deve ser maior que 0");
    assert!(BUFFER_SIZE > HEADER_SIZE, "Tamanho do buffer ({}) deve ser maior que o tamanho do cabeçalho ({})", BUFFER_SIZE, HEADER_SIZE);
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
        match agent.join() {
            Ok(_) => (),
            Err(_) => ()
        }
    }

}
