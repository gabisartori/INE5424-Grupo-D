use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex, mpsc::{RecvError, Sender, self}};
use std::thread;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use logger::log::SharedLogger;
use logger::log::Logger;
use logger::{debug_file, debug_println, initializate_files_and_folders};
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
    pub fn run(self: Arc<Self>, actions: Vec<Action>, test_id: usize) -> (u32, u32) {
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
                    return (0, 0);
                }
            }
        }
    
        // Channel for handling the death instruction
        let (death_tx, death_rx) = mpsc::channel();
        let death_tx_clone = death_tx.clone();
        
        // Spawns both threads
        let sender_clone = Arc::clone(&self);
        let listener_clone = Arc::clone(&self);
        
        let sender = thread::spawn(move ||
            sender_clone.creater(send_actions, death_tx_clone));
        let listener = thread::spawn(move ||
            listener_clone.receiver(receive_actions, death_tx, test_id));
        
        // Threads results
        let s_acertos;
        let r_acertos;

        // Waits for either any of the threads to send in the channel, meaning the agent should die
        // If both threads end without sending anything, the agent should wait for them to finish
        match death_rx.recv() {
            Ok((t, n)) => {
                // TODO: Descobirir o outro valor
                match t {
                    "C" => {
                        s_acertos = n;
                        r_acertos = 0;
                    },
                    "R" => {
                        s_acertos = 0;
                        r_acertos = n;
                    },
                e => {
                        debug_println!("Agent {}: Identificador {e}", self.id);
                        s_acertos = 0;
                        r_acertos = 0;
                    }
                }
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
        return (s_acertos, r_acertos);
    }

    /// Agent thread that always receives messages and checks if they are the expected ones from the selected test
    fn receiver(&self, actions: Vec<ReceiveAction>, death_tx: Sender<(&str, u32)>, test_id: usize) -> u32 {
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
                let _ = death_tx.send(("R", acertos));
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
                        let path = format!("tests/test_{test_id}/acertos_{}.txt", self.id);
                        debug_file!(path, &message);
                        acertos += 1;
                    } else {
                        let path = format!("tests/test_{test_id}/erros{}_{i}.txt", self.id);
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
    fn creater(&self, actions: Vec<SendAction>, death_tx: Sender<(&str, u32)>) -> u32 {
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
                    let _ = death_tx.send(("C", acertos));
                    break;
                }
            }
        }
        return acertos;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let main_len = 10;
    // Processo principal inicializando os agentes
    if args.len() == main_len {
        let tests = tests::all_tests();
        let test_num = tests.len();
        initializate_files_and_folders!(test_num);
        for (test_id, (test_name, test)) in tests.iter().enumerate() {
            let mut children = Vec::new();
            let agent_num = test.len();
            for i in 0..agent_num {
                let c = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(args[1..].as_ref()) // Passando o número de agentes
                    .arg(test_id.to_string()) // Passando o ID do teste
                    .arg(i.to_string()) // Passando o ID do agente
                    .spawn()
                    .expect(format!("Falha ao spawnar processo {i}").as_str());
                children.push(c);
            }
            // Aguardar a finalização de todos os agentes
            for mut c in children {
                c.wait().expect("Falha ao esperar processo filho");
            }
            let file_path = format!("tests/test_{test_id}/Resultado.txt");
            let file_path = file_path.as_str();
            let final_path = "tests/Resultado.txt";
            calculate_test(file_path, final_path, agent_num, test_name);
        }
    } else if args.len() == main_len + 2 {
        // Sub-processo: Execução do agente
        pub const IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        pub const PORT: u16 = 3000;
        let test_id: usize = args[main_len]
            .parse()
            .expect("Falha ao converter test_id para usize");
        let agent_id: usize = args.last().unwrap()
            .parse()
            .expect("Falha ao converter agent_id para usize");
        let (_, mut test) = tests::all_tests()[test_id].clone();
        let agent_num = test.len();
        
        let agent = create_agents(
            agent_id,
            agent_num,
            IP,
            PORT,
        );
        let actions = test.remove(agent_id);
        let (s_acertos, r_acertos) = match agent {
            Ok(agent) => agent.run(actions, test_id),
            Err(e) => panic!("Falha ao criar agente {}: {}", agent_id, e),
        };
        // Output the results to a file
        let file_path = format!("tests/test_{test_id}/Resultado.txt");
        let msg = format!("AGENTE {agent_id} -> ENVIOS: {s_acertos} - RECEBIDOS: {r_acertos}\n");
        debug_file!(file_path, &msg.as_bytes());
    } else {
        println!("uso: cargo run <broadcast> <timeout> <timeout_limit> <message_timeout> <broadcast_timeout> <gossip_rate> <w_size> <loss_rate> <corruption_rate>");
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
// TODO: Fazer com que ela receba o teste e compare os resultados
/// Reads and organizes the output file of all sub-processes in a test,
/// then writes the results to a final file.
/// The lines of output file must be formated as "AGENTE X -> ENVIOS: X - RECEBIDOS: X"
fn calculate_test(file_path: &str, final_path: &str, agent_num: usize, test_name: &str) {
    let file = File::open(file_path).expect("Erro ao abrir o arquivo de log");
    let mut reader = BufReader::new(file);

    let mut total_sends: u32 = 0;
    let mut total_receivs: u32 = 0;
    let mut line = String::new();
    // a vector to store the results, with a preset size
    let mut resultados: Vec<String> = vec![String::new(); agent_num];
    // for each line, extract the sends and receivs
    while BufRead::read_line(&mut reader, &mut line).unwrap() > 0 {
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
    let mut file: File = match OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)
    {
        Ok(f) => f,
        Err(e) => panic!("Erro ao abrir o arquivo: {}", e),
    };
    Write::write_all(&mut file, result_str.as_bytes())
        .expect("Erro ao escrever no arquivo");

    let mut file: File = match OpenOptions::new()
        .write(true)
        .append(true)
        .open(final_path)
    {
        Ok(f) => f,
        Err(e) => panic!("Erro ao abrir o arquivo: {}", e),
    };

    let result = format!("\n----------\nTeste {test_name}\n----------\nTotal de Mensagens Enviadas: {total_sends}\nTotal de Mensagens Recebidas: {total_receivs}\n");
    Write::write_all(&mut file, result.as_bytes())
        .expect("Erro ao escrever no arquivo");
}
