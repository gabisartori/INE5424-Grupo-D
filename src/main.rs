use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, mpsc::{RecvError, Sender, self}};
use std::{thread, vec};
use std::{fs::{File, OpenOptions}, io::{Write, BufRead, BufReader}};

use logger::log::SharedLogger;
use logger::log::Logger;
use logger::{debug_file, debug, initializate_folders};
use relcomm::reliable_communication::ReliableCommunication;
use relcomm::node::Node;
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
        let (sender_tx, sender_rx) = mpsc::channel();
        let (listener_tx, listener_rx) = mpsc::channel();

        // Spawns both threads
        let sender_clone = Arc::clone(&self);
        let listener_clone = Arc::clone(&self);

        let sender = thread::spawn(move ||
            sender_clone.creater(send_actions, death_tx_clone, sender_tx));
        let listener = thread::spawn(move ||
            listener_clone.receiver(receive_actions, death_tx, listener_tx, test_id));

        // Threads results
        let s_acertos;
        let r_acertos;

        // Waits for either any of the threads to send in the channel, meaning the agent should die
        // If both threads end without sending anything, the agent should wait for them to finish
        match death_rx.recv() {
            Ok((t, n)) => {
                match t {
                    "C" => {
                        s_acertos = n;
                        let mut t = 0;
                        while let Ok(r) = listener_rx.try_recv() {
                            t = r;
                        }
                        r_acertos = t;

                    },
                    "R" => {
                        r_acertos = n;
                        let mut t = 0;
                        while let Ok(r) = sender_rx.try_recv() {
                            t = r;
                        }
                        s_acertos = t;
                    },
                    e => {
                        debug!("Identificador Inválido {e}");
                        s_acertos = 0;
                        r_acertos = 0;
                    }
                }
            },
            Err(RecvError) => {
                r_acertos = match listener.join() {
                    Ok(r) => r,
                    Err(e) => {
                        debug!("Erro ao esperar thread listener encerrar: {:?}", e);
                        0
                    },
                };
                s_acertos = match sender.join() {
                    Ok(s) => s,
                    Err(e) => {
                        debug!("Erro ao esperar thread sender encerrar: {:?}", e);
                        0
                    },
                };
            }
        }
        return (s_acertos, r_acertos);
    }

    /// Agent thread that always receives messages and checks if they are the expected ones from the selected test
    fn receiver(&self, actions: Vec<ReceiveAction>, death_tx: Sender<(&str, u32)>,
                survival_tx: Sender<u32>, test_id: usize) -> u32 {
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
            let mut message = Vec::new();
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
                        let _ = survival_tx.send(acertos);
                    } else {
                        let path = format!("tests/test_{test_id}/erros{}_{i}.txt", self.id);
                        debug_file!(path, &message);
                    }
                },
                Err(e) => { debug!("Mensagem recebida não é uma string utf-8 válida: {}", e); },
            }
            i += 1;
        }
        return acertos;
    }

    /// Agent thread that sends preset messages from the selected test
    fn creater(&self, actions: Vec<SendAction>, death_tx: Sender<(&str, u32)>,
                survival_tx: Sender<u32>) -> u32 {
        let mut acertos = 0;
        for action in actions {
            match action {
                SendAction::Send { destination, message } => {
                    acertos += self.communication.send(destination, message.as_bytes().to_vec());
                    let _ = survival_tx.send(acertos);
                },
                SendAction::Broadcast { message } => {
                    acertos += self.communication.broadcast(message.as_bytes().to_vec());
                    let _ = survival_tx.send(acertos);
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
    let main_len = 1;
    // Processo principal inicializando os agentes
    if args.len() == main_len {
        let tests = tests::all_tests();
        let test_num = tests.len();
        let mut pass_tests = test_num;
        let final_path = "tests/Resultado.txt";
        initializate_folders!(test_num);
        for (test_id, (test_name, test)) in tests.iter().enumerate() {
            let mut children = Vec::new();
            let agent_num = test.len();
            for i in 0..agent_num {
                let c = std::process::Command::new(std::env::current_exe().unwrap())
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
            let expected = get_expected(test);
            if calculate_test(file_path, final_path, agent_num, test_name, test_id, expected) {
                pass_tests -= 1;
            }
        }
        debug_file!(final_path,
            format!("\n---------------------\nTestes passados: {pass_tests
            }/{test_num}\n").as_bytes());
    } else if args.len() == main_len + 2 {
        // Sub-processo: Execução do agente
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
        println!("uso: cargo run");
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
) -> Result<Arc<Agent>, std::io::Error> {
    let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port: u16 = 3000;
    let nodes: Vec<Node> = (0..agent_num)
        .map(|i| Node::new(SocketAddr::new(ip, port + (i as u16)), i))
        .collect();

    let logger = Arc::new(
        Logger::new(
            true,
            true,
            true,
            true,
            agent_num));

    let agent = Arc::new(
        Agent::new(id, nodes, logger)?
    );
    Ok(agent)
}

type Expected = (Vec<usize>, Vec<usize>, Vec<usize>);
/// Calculates the expected number of sends, receives and deaths for each agent in the test
fn get_expected(test: &Vec<Vec<Action>>) -> Expected {
    let agent_num: usize = test.len();
    let mut send_actions = vec![0; agent_num];
    let mut receive_actions = vec![0; agent_num];
    let mut die_actions = vec![0; agent_num];
    let mut rec_snd = 0;
    for (id, agent) in test.iter().enumerate() {
        for action in agent {
            match action {
                Action::Send(s) => {
                    match s {
                        SendAction::Send { .. } => {
                            send_actions[id] += 1;
                            rec_snd += 1;
                        },
                        SendAction::Broadcast { .. } => {
                            send_actions[id] += agent_num;
                            rec_snd += agent_num;
                        },
                        SendAction::DieAfterSend {} => {
                            die_actions[id] += 1;
                        }

                    }
                },
                Action::Receive(r) => {
                    match r {
                        ReceiveAction::Receive { .. } => {
                            receive_actions[id] += 1;
                            rec_snd -= 1;
                        },
                        ReceiveAction::DieAfterReceive { .. } => {
                            die_actions[id] += 1;
                        }
                    }
                },
                Action::Die() => {
                    die_actions[id] += 1;
                }
            }
        }
    }
    if rec_snd > 0 {
        for i in 0..agent_num {
            if send_actions[i] > 0 {
                send_actions[i] -= rec_snd
            }
        }
    }
    (send_actions, receive_actions, die_actions)
}

/// Reads and organizes the output file of all sub-processes in a test,
/// then writes the results to a final file.
/// The lines of output file must be formated as "AGENTE X -> ENVIOS: X - RECEBIDOS: X"
fn calculate_test(file_path: &str, final_path: &str, agent_num: usize,
                test_name: &str, test_id: usize, expected: Expected) -> bool {
    let file = File::open(file_path).expect("Erro ao abrir o arquivo de log");
    let mut reader = BufReader::new(file);

    let (expected_s, expected_r, expected_d) = expected;
    let mut deaths = 0;

    let mut errors: Vec<String> = vec![String::new(); agent_num];
    for idx in 0..agent_num {
        if expected_d[idx] != 0 {
            errors[0].push_str(&format!("Agente {idx} deveria morrer\n"));
            deaths += 1;
        }
    }

    let mut has_errors = false;

    let mut total_sends = 0;
    let mut total_receivs = 0;
    let mut line = String::new();
    // a vector to store the results, with a preset size
    let mut resultados: Vec<String> = vec![String::new(); agent_num];
    let mut total_expected_sends = 0;
    let mut total_expected_receivs = 0;
    // for each line, extract the sends and receivs
    while BufRead::read_line(&mut reader, &mut line).unwrap() > 0 {
        let words: Vec<&str> = line.split_whitespace().collect();
        let idx = words[1].parse::<usize>().unwrap();
        // saves the line as str on the correct index in resultados
        resultados[idx] = line.clone();
        // extract the sends and receivs from the line
        let sends: usize = words[4].parse().unwrap();
        let receivs: usize = words[7].parse().unwrap();
        // check if the sends and receivs match the expected values
        let exp = expected_s[idx];
        if sends != exp {
            if !(sends <= exp + deaths && exp <= sends + deaths) {
                errors[idx].push_str(&format!("Agente {idx
                }: Enviados: {sends
                } - Esperados: {
                }, com {deaths
                } mortes\n", expected_s[idx]));
                has_errors = true;
            }
        }
        let exp = expected_r[idx];
        if receivs != exp {
            if !(receivs <= exp + deaths && exp <= receivs + deaths) {
                errors[idx].push_str(&format!("Agente {idx
                    }: Recebidos: {receivs
                    } - Esperados: {
                    }, com {deaths
                    } mortes\n", expected_r[idx]));
                has_errors = true;
            }
        }
        total_sends += sends;
        total_receivs += receivs;
        total_expected_sends += expected_s[idx];
        total_expected_receivs += expected_r[idx];
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
    // write the final results to the final file
    let mut errors = errors.join("");
    if errors.len() > 0 {
        errors.push_str("----------\n");
    }
    let msg = format!("\n----------\nTeste {test_id
    }: {test_name
    }\n----------\n{errors
    }Total de Mensagens Enviadas: {total_sends
    }/{total_expected_sends
    }\nTotal de Mensagens Recebidas: {total_receivs
    }/{total_expected_receivs}\n");
    debug_file!(final_path, &msg.as_bytes());
    has_errors
}
