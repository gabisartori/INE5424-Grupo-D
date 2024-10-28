use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc::RecvError;
use std::sync::{Arc, Mutex};
use std::thread;
// use rand::Rng;

use logger::log::SharedLogger;
use logger::log::Logger;
use logger::{debug_file, debug_println};
use relcomm::reliable_communication::{Node, ReliableCommunication};
use tests::Action;

// Importa as configurações de endereços dos processos
mod tests;

struct Agent {
    id: usize,
    communication: Arc<ReliableCommunication>,
}

impl Agent {
    fn new(
        id: usize,
        nodes: Vec<Node>,
        // TODO: Update agent struct to match new test format
        _n_msgs: u32,
        logger: SharedLogger,
    ) -> Result<Self, std::io::Error> {
        Ok(Agent {
            id,
            communication: ReliableCommunication::new(
                nodes[id].clone(),
                nodes,
                logger,

            )?,
        })
    }

    fn receiver(&self, actions: Vec<Action>, msg_limit: u32, death_tx: std::sync::mpsc::Sender<u32>) -> u32 {
        let mut acertos = 0;
        let mut i = 0;
        loop {
            let mut message: Vec<u8> = Vec::new();
            if !self.communication.receive(&mut message) {
                break;
            }
            match String::from_utf8(message.clone()) {
                Ok(msg) => {
                    if actions.contains(&Action::Receive { message: msg.clone() }) {
                        acertos += 1;
                    } else {
                        let path = format!("tests/erros{}_{i}.txt", self.id);
                        debug_file!(path, &message);
                    }
                },
                Err(e) => { debug_println!("Agent {}: Mensagem recebida não é uma string utf-8 válida: {}", self.id, e); },
            }
            i += 1;
            if acertos == msg_limit {
                // Ignore send result because the run function cannot end until the receiver thread ends
                let _ = death_tx.send(acertos);
                break;
            }
        }
        return acertos;
    }

    fn creater(&self, actions: Vec<Action>) -> u32 {
        let mut acertos = 0;
        for action in actions {
            match action {
                Action::Send { destination, message } => {
                    acertos  += self.communication.send(destination, message.as_bytes().to_vec());
                },
                Action::Broadcast { message } => {
                    acertos += self.communication.broadcast(message.as_bytes().to_vec());
                },
                Action::Receive { .. } => { panic!("Agent {}: thread creater não deve receber ação de receber mensagem", self.id) },
                Action::Die { .. } => { panic!("Agent {}: thread creater não deve receber ação de morrer", self.id) },
            }
        }
        return acertos;
    }

    pub fn run(self: Arc<Self>, actions: Vec<Action>) {
        let mut send_actions = Vec::new();
        let mut receive_actions = Vec::new();
        let mut die = 0;
        for action in actions {
            match action {
                Action::Send { .. } | Action::Broadcast { .. } => {
                    send_actions.push(action);
                },
                Action::Receive { .. } => {
                    receive_actions.push(action);
                },
                Action::Die { after_n_messages } => die = after_n_messages,
            }
        }
    

        let sender_clone = Arc::clone(&self);
        let listener_clone = Arc::clone(&self);
        
        // Cria threads para enviar e receber mensagens e recupera o retorno delas
        let (death_tx, death_rx) = std::sync::mpsc::channel();
        let sender = thread::spawn(move || sender_clone.creater(send_actions));
        let listener = thread::spawn(move || listener_clone.receiver(receive_actions, die, death_tx));
        
        let s_acertos;
        let r_acertos;

        match death_rx.recv() {
            Ok(n) => {
                // listener recebeu instrução DIE, então a thread deve morrer
                r_acertos = n;
                s_acertos = 0;
            },
            Err(RecvError) => {
                // listener encerrou sem receber instrução DIE, esperar sender encerrar também
                r_acertos = match listener.join() {
                    Ok(r) => r,
                    Err(_) => {
                        debug_println!("Agent {}: Erro ao esperar thread listener encerrar", self.id);
                        0
                    },
                };
                s_acertos = match sender.join() {
                    Ok(s) => s,
                    Err(_) => {
                        debug_println!("Agent {}: Erro ao esperar thread sender encerrar", self.id);
                        0
                    },
                };
            }
        }
        let path = format!("tests/Resultado.txt");
        let msg = format!(
            "AGENTE {} -> ENVIOS: {s_acertos} - RECEBIDOS: {r_acertos}\n",
            self.id
        );
        debug_file!(path, &msg.as_bytes());
    }
}

fn create_agents(
    id: usize,
    agent_num: usize,
    n_msgs: u32,
    ip: IpAddr,
    port: u16,
) -> Result<Arc<Agent>, std::io::Error> {
    let mut nodes: Vec<Node> = Vec::new();

    // Contruir vetor unificando os nós locais e os remotos
    for i in 0..agent_num {
        nodes.push(Node::new(SocketAddr::new(ip, port + (i as u16)), i));
    }

    let logger = Arc::new(Mutex::new(Logger::new(1, agent_num)));

    let agent = Arc::new(
        Agent::new(
            id, nodes,
            n_msgs,
            logger,        
        )?
    );
    Ok(agent)
}

// TODO: Fix this function so it works with the new logger
fn calculate_test(agent_num: usize) {
    let file = std::fs::File::open("tests/Resultado.txt").expect("Erro ao abrir o arquivo de log");
    let mut reader = std::io::BufReader::new(file);

    let mut total_sends: u32 = 0;
    let mut total_receivs: u32 = 0;
    let mut line = String::new();
    // a vector to store the results, with a preset size
    let mut resultados: Vec<String> = vec![String::new(); agent_num];
    while std::io::BufRead::read_line(&mut reader, &mut line).unwrap() > 0 {
        let words: Vec<&str> = line.split_whitespace().collect();
        let sends: u32 = words[4].parse().unwrap();
        let receivs: u32 = words[7].parse().unwrap();
        total_sends += sends;
        total_receivs += receivs;
        let idx = words[1].parse::<u32>().unwrap() as usize;
        // saves the line as str on the correct index in resultados
        resultados[idx] = line.clone();
        line.clear();
    }
    // turn results into a string and write it to the file
    let mut result_str = String::new();
    for a in resultados {
        result_str.push_str(&a);
    }
    // clear the file and rewrite the results in order
    let mut file: std::fs::File = match std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open("tests/Resultado.txt")
    {
        Ok(f) => f,
        Err(e) => panic!("Erro ao abrir o arquivo: {}", e),
    };
    std::io::Write::write_all(&mut file, result_str.as_bytes())
        .expect("Erro ao escrever no arquivo");

    println!("Total de Mensagens Enviadas : {total_sends}");
    println!("Total de Mensagens Recebidas: {total_receivs}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut test = tests::broadcast_test_2();
    let agent_num = test.len();

    if args.len() == 14 {

        let mut childs = Vec::new();

        // Inicializar os agentes locais
        for i in 0..agent_num {
            let c = std::process::Command::new(std::env::current_exe().unwrap())
                .args(args[1..].as_ref()) // Passando o número de agentes
                .arg(i.to_string()) // Passando o ID do agente
                .spawn()
                .expect(format!("Falha ao spawnar processo {i}").as_str());
            childs.push(c);
        }
        // Aguardar a finalização de todos os agentes
        for mut c in childs {
            c.wait().expect("Falha ao esperar processo filho");
        }
        calculate_test(agent_num);

    } else if args.len() == 15 {
        // Se há 13 argumentos, então está rodando um subprocesso
        let n_msgs: u32 = args[2].parse().expect("Falha ao converter n_msgs para u32");
        let ip: IpAddr = args[8]
            .parse()
            .expect("Falha ao converter ip para IpAddr");
        let port: u16 = args[9]
            .parse()
            .expect("Falha ao converter port para u16");
        let agent_id: usize = args.last().unwrap()
            .parse()
            .expect("Falha ao converter agent_id para u32");
        
        let agent = create_agents(
            agent_id,
            agent_num,
            n_msgs,
            ip,
            port,
        );
        let actions = test.remove(agent_id);
        match agent {
            Ok(agent) => agent.run(actions),
            Err(e) => panic!("Falha ao criar agente {}: {}", agent_id, e),
        }
    } else {
        println!("uso: cargo run <agent_num> <n_msgs> <broadcast> <timeout> <timeout_limit> <message_timeout> <broadcast_timeout> <ip> <port> <gossip_rate> <w_size> <buffer_size> <loss_rate> <corruption_rate>");
        println!("enviado {:?}", args);
        panic!("Número de argumentos {} inválido", args.len());
    }
}
