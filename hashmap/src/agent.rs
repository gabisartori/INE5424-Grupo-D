use std::sync::Arc;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};


use logger::log::{SharedLogger, Logger};
use logger::debug;

use relcomm::reliable_communication::ReliableCommunication;
use relcomm::node::Node;
use crate::hashmap::DistrHash;
use crate::config::{KEYS, MSG_NUM, MSG_SIZE, WRITE_READ_RATIO};

pub struct Agent {
    id: usize,
    hash_table: Arc<DistrHash>,
}

impl Agent {
    fn new(
        id: usize,
        nodes: Vec<Node>,
        logger: SharedLogger,
    ) -> Result<Self, std::io::Error> {
        let communication = ReliableCommunication::new(
                nodes[id].clone(),
                nodes,
                logger,
            )?;
        Ok(Agent {
            id,
            hash_table: DistrHash::new(communication),
        })
    }

    pub fn run(&self) -> std::time::Duration {
        let start = std::time::Instant::now();
        for _ in 0..MSG_NUM {
            // Decide if it will read or write
            let key: String = Agent::get_random_key();
            if rand::random::<f32>() < WRITE_READ_RATIO {
                let msg = Agent::get_rnd_msg(MSG_SIZE);
                match self.hash_table.write(&key, &msg) {
                    Ok(_) => println!("Agent {} wrote key {}", self.id, key),
                    Err(e) => println!("Error: {}", e),
                }
            } else {
                match self.hash_table.read(&key) {
                    Some(msg) => println!("Agent {} read key {}: {}", self.id, key, msg),
                    None => println!("Agent {} could not read key {}", self.id, key),
                }
            }
            
        }
        if self.id == 0 { std::thread::sleep(std::time::Duration::from_secs(20)); }
        debug!("->-> Agente {} finished", self.id);
        start.elapsed()
    }

    fn get_random_key() -> String {
        let x = rand::random::<u8>() % KEYS.len() as u8;
        KEYS[x as usize].to_string()        
    }

    fn get_rnd_msg(size: usize) -> String {
        let mut msg = Vec::new();
        let mut x;
        for _ in 0..size {
            x = b":"[0];
            while x == b":"[0] { x = rand::random::<u8>()%26+97; }
            msg.push(x);
        }
        String::from_utf8(msg).unwrap()
    }
}

/// Creates the vector of nodes for all of the members of the group
/// Since this is currently always being tested in the same machine, the IP is always the same and the ports are based on the node id
/// Then it creates and returns the agent for the node that matches the sub-process id
pub fn create_agents(
    id: usize,
    agent_num: usize,
) -> Result<Agent, std::io::Error> {
    let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port: u16 = 3000;
    let nodes: Vec<Node> = (0..agent_num)
        .map(|i| Node::new(
            SocketAddr::new(
                ip, port + (i as u16)),
                 i))
        .collect();

    let logger = Arc::new(
        Logger::new(
            true,
            true,
            true,
            true,
            agent_num));

    let agent = Agent::new(id, nodes, logger)?;
    Ok(agent)
}
