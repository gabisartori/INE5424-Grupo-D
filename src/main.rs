#[macro_export]
macro_rules! debug_println {
    // This pattern accepts format arguments like println!
    ($($arg:tt)*) => {
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
use std::sync::Arc;
use std::thread;
// use rand::Rng;

mod lib {
    pub mod reliable_communication;
    pub mod channels;
    pub mod failure_detection;
    pub mod packet;
    pub mod flags;
}
use lib::reliable_communication::{ReliableCommunication, Node};

// Importa as configurações de endereços dos processos
mod config;
use config::{Broadcast, AGENT_NUM, BROADCAST, LOCALHOST, PORT,MSG, N_MSGS};

struct Agent {
    id: usize,
    communication: Arc<ReliableCommunication>
}

impl Agent {
    fn new(id: usize, nodes: Vec<Node>) -> Self {
        Agent {
            id,
            communication: ReliableCommunication::new(nodes[id].clone(), nodes)
        }
    }

    fn receiver(&self) -> u32 {
        let mut acertos = 0;
        let mut i = 0;
        loop {
            let mut message: Vec<u8> = Vec::new();
            if !self.communication.receive(&mut message) {
                break;
            }
            let gabarito: Vec<u8> = MSG.to_string().as_bytes().to_vec();
            if self.compare_msg(&message, &gabarito) {
                acertos += 1;
            } else {
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
            i += 1;
        }
        return acertos;
    }

    fn creater(&self) -> u32 {
        let mut acertos= 0;
        for i in 0..N_MSGS {
            // Send message to the selected node
            let msg: Vec<u8> = MSG.to_string().as_bytes().to_vec();

            let tot = self.communication.broadcast(msg);
            acertos += tot;
            if tot == 0 {
                debug_println!("ERROR -> AGENTE {} TIMED OUT AO TENTAR ENVIAR A MENSAGEM {}", self.id, i);
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

    pub fn run(self: Arc<Self>) {
        let sender_clone = Arc::clone(&self);
        let listener_clone = Arc::clone(&self);
        // Cria threads para enviar e receber mensagens e recupera o retorno delas
        let sender = thread::spawn(move || sender_clone.creater());
        let listener = thread::spawn(move || listener_clone.receiver());
        let s_acertos =  sender.join().unwrap();
        let r_acertos = listener.join().unwrap();
        let max = if BROADCAST == Broadcast::NONE {N_MSGS} else {N_MSGS*(AGENT_NUM as u32)};
        let path = format!("tests/Resultado.txt");
        let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(path) {
            Ok(f) => f,
            Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
        };
        // println!("AGENTE {} -> ENVIOS: {s_acertos}/{max} - RECEBIDOS: {r_acertos}/{max}", self.id);
        let msf = format!("AGENTE {} -> ENVIOS: {s_acertos}/{max} - RECEBIDOS: {r_acertos}/{max}\n", self.id);
        std::io::Write::write_all(&mut file, msf.as_bytes()).expect("Erro ao escrever no arquivo");
    }
}


fn create_agents(id: usize) -> Arc<Agent> {
    let mut nodes: Vec<Node> = Vec::new();

    // Contruir vetor unificando os nós locais e os remotos
    for i in 0..AGENT_NUM {
        nodes.push(Node{addr: SocketAddr::new(LOCALHOST, 
            PORT + (i as u16)),
            agent_number: i});
    }
    
    let agent = Arc::new(Agent::new(id, nodes));
    agent

}

fn calculate_test() {
    let file = std::fs::File::open("tests/Resultado.txt").expect("Erro ao abrir o arquivo de log");
    let mut reader = std::io::BufReader::new(file);

    let mut total_sends: u32 = 0;
    let mut total_receivs: u32 = 0;
    let mut expected_sends: u32 = 0;
    let mut line = String::new();
    // a vector to store the results, with a preset size
    let mut resultados: Vec<String> = vec![String::new(); AGENT_NUM as usize];
    while std::io::BufRead::read_line(&mut reader, &mut line).unwrap() > 0 {
        let words: Vec<&str> = line.split_whitespace().collect();
        let sends: Vec<u32> = words[4].split("/").map(|x| x.parse().unwrap()).collect();
        let receivs: Vec<u32> = words[7].split("/").map(|x| x.parse().unwrap()).collect();
        total_sends += sends[0];
        total_receivs += receivs[0];
        expected_sends += sends[1];
        let idx = words[1].parse::<u32>().unwrap() as usize;
        // saves the line as str on the correct index in resultados
        resultados[idx] = line.clone();
        line.clear();
    }
    // clear the file and rewrite the results in order
    let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                        .write(true)
                                        .truncate(true)
                                        .open("tests/Resultado.txt") {
        Ok(f) => f,
        Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
    };
    for a in resultados {
        std::io::Write::write_all(&mut file, a.as_bytes()).expect("Erro ao escrever no arquivo");
    }
    println!("Total de Mensagens Enviadas : {total_sends}/{expected_sends}");
    println!("Total de Mensagens Recebidas: {total_receivs}/{expected_sends}");
}


fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Verifica se o programa foi executado com argumentos (quando for rodado como um subprocesso)
    if args.len() > 1 {
        let agent_id: usize = args[1].parse().expect("Falha ao converter agent_id para u32");
        let agent = create_agents(agent_id);
        agent.run();
    } else { // Se não há argumentos, então está rodando o processo principal
        assert!(AGENT_NUM > 0, "Número de agentes deve ser maior que 0");
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
        calculate_test();
    }
}
