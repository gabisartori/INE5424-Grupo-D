use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc::{RecvError, Sender, self};
use std::sync::{Arc, Mutex};
use std::thread;
// use rand::Rng;

use logger::log::SharedLogger;
use logger::log::Logger;
use logger::{debug_file, debug_println};
use relcomm::reliable_communication::{Node, ReliableCommunication};
use tests::{Action, ReceiveAction, SendAction};

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

    /// Start both creater and receiver threads
    /// Also runs some logic for if any of the threads is supposed to trigger the death of the entire agent in the test
    pub fn run(self: Arc<Self>, actions: Vec<Action>) {
        // Builds the instruction vectors
        let mut send_actions = Vec::new();
        let mut receive_actions = Vec::new();
        for action in actions {
            match action {
                Action::Send(action) => {
                    send_actions.push(action);
                },
                Action::Receive(action) => {
                    receive_actions.push(action);
                },
                // Die instruction means the agent shouldn't even start running
                Action::Die() => {
                    let path = format!("tests/Resultado.txt");
                    let msg = format!(
                        "AGENTE {} -> ENVIOS: 0 - RECEBIDOS: 0\n",
                        self.id
                    );
                    debug_file!(path, &msg.as_bytes());
                    return;
                }
            }
        }
    
        // Channel for handling the death instruction
        let (death_tx, death_rx) = mpsc::channel();
        let death_tx_clone = death_tx.clone();
        
        // Spawns both threads
        let sender_clone = Arc::clone(&self);
        let listener_clone = Arc::clone(&self);
        
        let sender = thread::spawn(move || sender_clone.creater(send_actions, death_tx_clone));
        let listener = thread::spawn(move || listener_clone.receiver(receive_actions, death_tx));
        
        // Threads results
        let s_acertos;
        let r_acertos;

        // Waits for either any of the threads to send in the channel, meaning the agent should die
        // If both threads end without sending anything, the agent should wait for them to finish
        match death_rx.recv() {
            Ok(_n) => {
                // TODO: Decidir o que fazer com o valor recebido, se foi enviado pela creater ou pela receiver
                r_acertos = 0;
                s_acertos = 0;
            },
            Err(RecvError) => {
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
        
        // Output the results to a file
        let msg = format!(
            "AGENTE {} -> ENVIOS: {s_acertos} - RECEBIDOS: {r_acertos}\n",
            self.id
        );
        debug_file!("tests/Resultado.txt", &msg.as_bytes());
    }

    /// Agent thread that always receives messages and checks if they are the expected ones from the selected test
    fn receiver(&self, actions: Vec<ReceiveAction>, death_tx: Sender<u32>) -> u32 {
        let mut acertos = 0;
        let mut i = 0;
        let mut expected_messages = Vec::new();
        let mut msg_limit = u32::MAX;
        for action in actions {
            match action {
                ReceiveAction::Receive { message } => { expected_messages.push(message); },
                ReceiveAction::DieAfterReceive { after_n_messages } => { msg_limit = after_n_messages; },
            }
        }
        loop {
            // Die if the message limit is reached
            if acertos >= msg_limit {
                // Ignore send result because the run function cannot end until the receiver thread ends
                let _ = death_tx.send(acertos);
                break;
            }
            // Receive the next message
            let mut message: Vec<u8> = Vec::new();
            if !self.communication.receive(&mut message) {
                break;
            }
            // Check if the message is the expected one
            match String::from_utf8(message.clone()) {
                Ok(msg) => {
                    if expected_messages.contains(&msg) {
                        let path = format!("tests/acertos_{}.txt", self.id);
                        debug_file!(path, &message);
                        acertos += 1;
                    } else {
                        let path = format!("tests/erros{}_{i}.txt", self.id);
                        debug_file!(path, &message);
                    }
                },
                Err(e) => { debug_println!("Agent {}: Mensagem recebida não é uma string utf-8 válida: {}", self.id, e); },
            }
            i += 1;
        }
        return acertos;
    }

    /// Agent thread that sends preset messages from the selected test
    fn creater(&self, actions: Vec<SendAction>, death_tx: Sender<u32>) -> u32 {
        let mut acertos = 0;
        for action in actions {
            match action {
                SendAction::Send { destination, message } => {
                    acertos += self.communication.send(destination, message.as_bytes().to_vec());
                },
                SendAction::Broadcast { message } => {
                    acertos += self.communication.broadcast(message.as_bytes().to_vec());
                },
                SendAction::DieAfterSend {} => {
                    // Ignore send result because the run function cannot end until the receiver thread ends
                    let _ = death_tx.send(acertos);
                    break;
                }
            }
        }
        return acertos;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut test = tests::broadcast_test_6();
    let agent_num = test.len();

    if args.len() == 14 {
        // Processo principal inicializando os agentes
        let mut children = Vec::new();
        for i in 0..agent_num {
            let c = std::process::Command::new(std::env::current_exe().unwrap())
                .args(args[1..].as_ref()) // Passando o número de agentes
                .arg(i.to_string()) // Passando o ID do agente
                .spawn()
                .expect(format!("Falha ao spawnar processo {i}").as_str());
            children.push(c);
        }
        // Aguardar a finalização de todos os agentes
        for mut c in children {
            c.wait().expect("Falha ao esperar processo filho");
        }
        calculate_test(agent_num);
    } else if args.len() == 15 {
        // Sub-processo: Execução do agente
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

/// Creates the vector of nodes for all of the members of the group
/// Since this is currently always being tested in the same machine, the IP is always the same and the ports are based on the node id
/// Then it creates and returns the agent for the node that matches the sub-process id
fn create_agents(
    id: usize,
    agent_num: usize,
    ip: IpAddr,
    port: u16,
) -> Result<Arc<Agent>, std::io::Error> {
    let nodes: Vec<Node> = (0..agent_num)
        .map(|i| Node::new(SocketAddr::new(ip, port + (i as u16)), i))
        .collect();

    let logger = Arc::new(Mutex::new(Logger::new(1, agent_num)));

    let agent = Arc::new(
        Agent::new(id, nodes, logger)?
    );
    Ok(agent)
}

// TODO: Comentar melhor essa função
/// Reads and organizes the output of all sub-processes
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
