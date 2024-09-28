#[macro_export]
macro_rules! debug_println {
    // This pattern accepts format arguments like println!
    ($($arg:tt)*) => {
//        if cfg!(debug_assertions) {
        let path = format!("tests/debug.txt");
        let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(path) {
            Ok(f) => f,
            Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
        };
        let msf = format!("----------\n{}\n----------\n", format!($($arg)*));
        std::io::Write::write_all(&mut file, msf.as_bytes()).expect("Erro ao escrever no arquivo");
    };
}

use std::net::SocketAddr;
use std::thread;
use rand::Rng;
use std::env;

mod lib {
    pub mod reliable_communication;
    pub mod channels;
    pub mod failure_detection;
    pub mod packet;
    pub mod flags;
    pub mod message_queue;
}
use lib::reliable_communication::ReliableCommunication;
use lib::packet::HEADER_SIZE;

// Importa as configurações de endereços dos processos
mod config;
use config::{Node, BUFFER_SIZE, NODES, AGENT_NUM, N_MSGS};

#[derive(Clone)]
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

    fn listener(&self) -> u128 {
        let mut acertos: u128 = 0;
        let stop = if !cfg!(debug_assertions) { N_MSGS } else { N_MSGS*AGENT_NUM };
        for i in 0..stop
        {
            let mut message: Vec<u8> = Vec::new();
            if !self.communication.receive(&mut message) {
                break;
            }
            let gabarito: Vec<u8> = config::MSG.to_string().as_bytes().to_vec();
            if self.compare_msg(&message, &gabarito) {
                acertos += 1;
            } else {
                debug_println!("ERRO -> AGENTE {} RECEBEU A MENSAGEM {i} INCORRETA", self.id);
                let path = format!("tests/erros{}_{i}.txt", self.id);
                let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                                    .create(true)
                                                    .append(true)
                                                    .open(path) {
                    Ok(f) => f,
                    Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
                };
                // connverts the message to a string
                std::io::Write::write_all(&mut file, &message).expect("Erro ao escrever no arquivo");
            }
        }
        return acertos;
    }

    fn sender(&self) -> u128 {
        let mut acertos: u128 = 0;
        for i in 0..N_MSGS {
            let destination: u32 = self.pick_destination();
            let dst_addr: SocketAddr = self.communication.group[destination as usize].addr;
            // Send message to the selected node
            let msg: Vec<u8> = config::MSG.to_string().as_bytes().to_vec();

            if self.communication.send(&dst_addr, msg) {
                acertos += 1;
            } else {
                debug_println!("ERROR -> AGENTE {} TIMED OUT AO TENTAR ENVIAR A MENSAGEM {i} PARA AGENTE {destination}", self.id);
            }
        }
        return acertos;
    }

    fn compare_msg(&self, msg1: &Vec<u8>, msg2: &Vec<u8>) -> bool {
        if msg1.len() != msg2.len() {
            return false;
        }
        for i in 0..msg1.len() {
            if msg1[i] != msg2[i] {
                return false;
            }
        }
        true
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

    pub fn run(self) {
        let sender_clone = self.clone();
        let listener_clone = self.clone();
        // Cria threads para enviar e receber mensagens e recupera o retorno delas
        let sender = thread::spawn(move || sender_clone.sender());
        let listener = thread::spawn(move || listener_clone.listener());
        let s_acertos =  sender.join().unwrap();
        let r_acertos = listener.join().unwrap();
        let max = N_MSGS as u128;
        println!("AGENTE {} -> ENVIOS: {s_acertos}/{max} - RECEBIDOS: {r_acertos}/{max}", self.id);
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();

    // Verifica se o programa foi executado com argumentos (quando for rodado como um subprocesso)
    if args.len() > 1 {
        let agent_id: u32 = args[1].parse().expect("Falha ao converter agent_id para u32");
        let agent = create_agents(agent_id);
        agent.run();
    } else { // Se não há argumentos, então está rodando o processo principal
        assert!(AGENT_NUM > 0, "Número de agentes deve ser maior que 0");
        assert!(BUFFER_SIZE > HEADER_SIZE, "Tamanho do buffer ({BUFFER_SIZE}) deve ser maior que o tamanho do cabeçalho ({HEADER_SIZE})");  
        let mut childs = Vec::new();  
        // Inicializar os agentes locais
        for i in 0..AGENT_NUM {
            let c = std::process::Command::new(std::env::current_exe().unwrap())
                .arg(i.to_string())  // Passando o ID do agente
                .spawn()
                .expect(format!("Falha ao spawnar processo {i}").as_str());
            childs.push(c);
        }
        // Aguardar a finalização de todos os agentes
        for mut c in childs {
            c.wait().expect("Falha ao esperar processo filho");
        }
    }
}

fn create_agents(id: u32) -> Agent  {
    let mut nodes: Vec<Node> = Vec::new();

    // Contruir vetor unificando os nós locais e os remotos
    for i in 0..AGENT_NUM {
        nodes.push(Node{addr: SocketAddr::new(config::LOCALHOST, 3100 + (i as u16)), agent_number: i});
    }

    if let Some(remote_nodes) = NODES {
        for node in remote_nodes {
            nodes.push(Node{addr: node.addr, agent_number: node.agent_number});
        }
    }
    let agent = Agent::new(id, SocketAddr::new(config::LOCALHOST, 3100 + id as u16), nodes.clone());
    agent

}
