mod config;
mod hashmap;
mod agent;
mod formatter;

use logger::{initializate_folders, debug_file};

use config::{AGENT_NUM, TESTS_NUM};
use agent::create_agents;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Processo principal inicializando os agentes
    if args.len() == 1 {
        initializate_folders!(TESTS_NUM);
        for t in 0..TESTS_NUM {
            let mut children = Vec::new();
            for i in 0..(t+5) {
                let c = std::process::Command::new(std::env::current_exe().unwrap())
                    .arg(t.to_string()) // Passando o ID do teste
                    .arg(i.to_string()) // Passando o ID do agente
                    .spawn()
                    .expect(format!("Falha ao spawnar processo {i}").as_str());
                children.push(c);
            }
            // Aguardar a finalização de todos os agentes
            for mut c in children {
                c.wait().expect("Falha ao esperar processo filho");
            }
        }
    } else if args.len() == 3 {
        // Sub-processo: Execução do agente
        let test_id: usize = args[1]
            .parse()
            .expect("Falha ao converter test_id para usize");
        let agent_id: usize = args[2]
            .parse()
            .expect("Falha ao converter agent_id para usize");

        let agent = create_agents(
            agent_id,
            AGENT_NUM,
        );
        let time = match agent {
            Ok(agent) => agent.run(),
            Err(e) => panic!("Falha ao criar agente {}: {}", agent_id, e),
        };
        // Output the results to a file
        let file_path = format!("tests/test_{test_id}/Resultado.txt");
        let msg = format!("AGENTE {agent_id} -> {} seconds to broadcast {} messages\n", time.as_secs_f32(),
        config::MSG_NUM);
        println!("{}", msg);
        debug_file!(file_path, &msg.as_bytes());
    } else {
        println!("uso: cargo run");
        println!("enviado {:?}", args);
        panic!("Número de argumentos {} inválido", args.len());
    }
}
