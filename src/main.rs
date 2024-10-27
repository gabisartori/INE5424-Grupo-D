use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::thread;
// use rand::Rng;

use logger::log::Logger;
use logger::{debug_file, debug_println, log};
use relcomm::reliable_communication::{Broadcast, Node, ReliableCommunication};

// Importa as configurações de endereços dos processos
mod tests;

struct Agent {
    id: usize,
    communication: Arc<ReliableCommunication>,
    n_msgs: u32,
}

impl Agent {
    fn new(
        id: usize,
        nodes: Vec<Node>,
        n_msgs: u32,
        timeout: u64,
        message_timeout: u64,
        timeout_limit: u32,
        w_size: usize,
        gossip_rate: usize,
        broadcast: Broadcast,
        broadcast_timeout: u64,
        logger: Logger,
    ) -> Self {
        Agent {
            id,
            communication: ReliableCommunication::new(
                nodes[id].clone(),
                nodes,
                timeout,
                message_timeout,
                timeout_limit,
                w_size,
                gossip_rate,
                broadcast,
                broadcast_timeout,
                logger,
            ),
            n_msgs,
        }
    }

    fn receiver(&self, actions: Vec<tests::Action>) -> u32 {
        let mut acertos = 0;
        let mut i = 0;
        loop {
            let mut message: Vec<u8> = Vec::new();
            if !self.communication.receive(&mut message) {
                break;
            }
            debug_println!("Agent {}: Recebida mensagem {}", self.id, String::from_utf8(message.clone()).unwrap());
            if actions.contains(&tests::Action::Receive { message: String::from_utf8(message.clone()).unwrap() }) {
                acertos += 1;
            } else {
                let path = format!("tests/erros{}_{i}.txt", self.id);
                debug_file!(path, &message);
            }
            i += 1;
        }
        return acertos;
    }

    fn creater(&self, actions: Vec<tests::Action>) -> u32 {
        let mut acertos = 0;
        for action in actions {
            match action {
                tests::Action::Send { destination, message } => {
                    let destination = &self.communication.group.lock().unwrap()[destination];
                    acertos  += self.communication.send(&destination.addr, message.as_bytes().to_vec());
                },
                tests::Action::Broadcast { message } => {
                    acertos += self.communication.broadcast(message.as_bytes().to_vec());
                },
                tests::Action::Receive { .. } => { panic!("Agent {}: thread creater não deve receber ação de receber mensagem", self.id) },
            }
        }
        return acertos;
    }

    pub fn run(self: Arc<Self>, actions: Vec<tests::Action>) {
        let mut send_actions = Vec::new();
        let mut receive_actions = Vec::new();

        for action in actions {
            match action {
                tests::Action::Send { .. } | tests::Action::Broadcast { .. } => {
                    send_actions.push(action);
                },
                tests::Action::Receive { .. } => {
                    receive_actions.push(action);
                },
            }
        }

        let sender_clone = Arc::clone(&self);
        let listener_clone = Arc::clone(&self);
        // Cria threads para enviar e receber mensagens e recupera o retorno delas
        let sender = thread::spawn(move || sender_clone.creater(send_actions));
        let listener = thread::spawn(move || listener_clone.receiver(receive_actions));
        let s_acertos = sender.join().unwrap();
        let r_acertos = listener.join().unwrap();
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
    broadcast: Broadcast,
    timeout: u64,
    timeout_limit: u32,
    message_timeout: u64,
    broadcast_timeout: u64,
    ip: IpAddr,
    port: u16,
    gossip_rate: usize,
    w_size: usize,
) -> Arc<Agent> {
    let mut nodes: Vec<Node> = Vec::new();

    // Contruir vetor unificando os nós locais e os remotos
    for i in 0..agent_num {
        nodes.push(Node::new(SocketAddr::new(ip, port + (i as u16)), i));
    }
    let mut logger: Logger = Logger::new(1, agent_num);

    let agent = Arc::new(
        Agent::new(
            id, nodes,
            n_msgs,
            timeout,
            message_timeout,
            timeout_limit,
            w_size,
            gossip_rate,
            broadcast,
        broadcast_timeout,
        logger,
    ));
    agent
}

fn calculate_test(agent_num: usize, n_msgs: usize, broadcast: &str) {
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
    let expected = match broadcast {
        "NONE" => agent_num * n_msgs,
        _ => agent_num * agent_num * n_msgs,
    };
    println!("Total de Mensagens Enviadas : {total_sends}/{expected}");
    println!("Total de Mensagens Recebidas: {total_receivs}/{expected}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut test = tests::send_test_2();
    let agent_num = test.len();

    if args.len() == 12 {

        assert!(agent_num > 0, "Número de agentes deve ser maior que 0");

        let mut childs = Vec::new();

        // Inicializar os agentes locais
        for i in 0..agent_num {
            let c = std::process::Command::new(std::env::current_exe().unwrap())
                .arg(args[1].clone()) // Passando o número de agentes
                .arg(args[2].clone()) // Passando o número de mensagens
                .arg(args[3].clone()) // Passando o tipo de broadcast
                .arg(args[4].clone()) // Passando o timeout
                .arg(args[5].clone()) // Passando o message_timeout
                .arg(args[6].clone()) // Passando o broadcast_timeout
                .arg(args[7].clone()) // Passando o IP
                .arg(args[8].clone()) // Passando a Porta base
                .arg(args[9].clone()) // Passando a taxa de gossip
                .arg(args[10].clone()) // Passando o tamanho da janela
                .arg(args[11].clone()) // Passando o limite de timeouts consecutivos
                .arg(i.to_string()) // Passando o ID do agente
                .spawn()
                .expect(format!("Falha ao spawnar processo {i}").as_str());
            childs.push(c);
        }
        // Aguardar a finalização de todos os agentes
        for mut c in childs {
            c.wait().expect("Falha ao esperar processo filho");
        }
        // calculate_test(agent_num, n_msgs, args[3].as_str());

    } else if args.len() == 13 {
        // Se há 13 argumentos, então está rodando um subprocesso
        let n_msgs: u32 = args[2].parse().expect("Falha ao converter n_msgs para u32");
        let broadcast: Broadcast = match args[3].as_str() {
            "NONE" => Broadcast::NONE,
            "BEB" => Broadcast::BEB,
            "URB" => Broadcast::URB,
            "AB" => Broadcast::AB,
            _ => panic!("Falha ao converter broadcast {} para Broadcast", args[3]),
        };
        let timeout: u64 = args[4]
            .parse()
            .expect("Falha ao converter timeout para u64");
        let message_timeout: u64 = args[5]
            .parse()
            .expect("Falha ao converter message_timeout para u64");
        let broadcast_timeout: u64 = args[6]
            .parse()
            .expect("Falha ao converter broadcast_timeout para u64");
        let ip: IpAddr = args[7].parse().expect("Falha ao converter ip para IpAddr");
        let port: u16 = args[8].parse().expect("Falha ao converter port para u16");
        let gossip_rate: usize = args[9]
            .parse()
            .expect("Falha ao converter gossip_rate para usize");
        let w_size: usize = args[10]
            .parse()
            .expect("Falha ao converter w_size para usize");
        let timeout_limit: u32 = args[11]
            .parse()
            .expect("Falha ao converter timeout_limit para u32");
        let agent_id: usize = args[12]
            .parse()
            .expect("Falha ao converter agent_id para u32");

        let agent = create_agents(
            agent_id,
            agent_num,
            n_msgs,
            broadcast,
            timeout,
            timeout_limit,
            message_timeout,
            broadcast_timeout,
            ip,
            port,
            gossip_rate,
            w_size,
        );
        let actions = test.remove(agent_id);
        agent.run(actions);
    } else {
        println!("uso: cargo run <agent_num> <n_msgs> <broadcast> <timeout> <message_timeout> <broadcast_timeout> <ip> <port> <gossip_rate> <w_size> <buffer_size> <timeout_limit>");
        println!("enviado {:?}", args);
        panic!("Número de argumentos inválido");
    }
}
