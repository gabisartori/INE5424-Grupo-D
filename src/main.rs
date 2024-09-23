#[macro_export]
macro_rules! debug_println {
    // This pattern accepts format arguments like println!
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!("----------\n{}\n----------\n", format!($($arg)*));  // Add 2 line breaks before and after
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    };
}

use std::net::SocketAddr;
use std::thread;
use std::sync::Arc;
use rand::Rng;

mod lib {
    pub mod reliable_communication;
    pub mod channels;
    pub mod failure_detection;
    pub mod packet;
    pub mod flags;
}
use lib::reliable_communication::ReliableCommunication;
use lib::packet::HEADER_SIZE;

// Importa as configurações de endereços dos processos
mod config;
use config::{Node, BUFFER_SIZE, NODES, AGENT_NUM, N_MSGS};

struct Agent {
    id: u32,
    communication: ReliableCommunication
}

impl Agent {
    fn new(id: u32, addr: SocketAddr, nodes: Vec<Node>) -> Self {
        Agent {
            id,
            communication: ReliableCommunication::new(addr, nodes)
        }
    }

    fn listener(&self) {
        let stop = if !cfg!(debug_assertions) { N_MSGS } else { N_MSGS*AGENT_NUM };        
        for _ in 0..stop
        {
            let mut message: Vec<u8> = Vec::new();
            if !self.communication.receive(&mut message) {
                break;
            }
            let msg = String::from_utf8_lossy(&message);
            let msf = format!("Agent {} received Message:\n-->\n{}", self.id, msg);
            debug_println!("MENSAGEM RECEBIDA POR AGENTE {}", self.id);
            if cfg!(debug_assertions) {
                // write message to a listener.txt file
                let path = format!("tests/listener_{}.txt", self.id);
                let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                                    .create(true)
                                                    .append(true)
                                                    .open(path) {
                    Ok(f) => f,
                    Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
                };
                std::io::Write::write_all(&mut file, msf.as_bytes())
                .expect("Erro ao escrever no arquivo");
            } else { println!("{}", msf); }
        }
    }

    fn sender(&self) {
        for i in 0..N_MSGS {
            let destination = self.pick_destination();
            let dst_addr = self.communication.group[destination as usize].addr;
            // Send message to the selected node
            let msg = config::MSGS[(i%3) as usize].to_string().as_bytes().to_vec();

            if self.communication.send(&dst_addr, msg) {
                    debug_println!("AGENTE {} ENVIOU A MENSAGEM PARA AGENTE {}", self.id, destination);
            } else {
                debug_println!("AGENTE {} TIMED OUT AO TENTAR ENVIAR A MENSAGEM PARA AGENTE {}", self.id, destination);
            }
        }
    }

    fn pick_destination(&self) -> u32 {
        if cfg!(debug_assertions) {
            let size = self.communication.group.len() as u32;
            let dst = rand::thread_rng().gen_range(0..size);
            if dst  == self.id { (dst + 1) % size }
            else { dst }
        } else {
            (self.id + 1) % self.communication.group.len() as u32
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
    assert!(AGENT_NUM > 0, "Número de agentes deve ser maior que 0");
    assert!(BUFFER_SIZE > HEADER_SIZE, "Tamanho do buffer ({}) deve ser maior que o tamanho do cabeçalho ({})", BUFFER_SIZE, HEADER_SIZE);
    let mut nodes: Vec<Node> = Vec::new();
    let mut local_agents: Vec<thread::JoinHandle<()>> = Vec::new();

    // Contruir vetor unificando os nós locais e os remotos
    for i in 0..AGENT_NUM {
        nodes.push(Node{addr: SocketAddr::new(config::LOCALHOST, 3100 + (i as u16)), agent_number: i});
    }

    if let Some(remote_nodes) = NODES {
        for node in remote_nodes {
            nodes.push(Node{addr: node.addr, agent_number: node.agent_number});
        }
    }

    // Inicializar os agentes locais
    for i in 0..AGENT_NUM {
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
