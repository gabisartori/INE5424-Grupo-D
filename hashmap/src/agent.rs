use std::thread;
use std::sync::Arc;
use std::time::Duration;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};


use logger::log::SharedLogger;
use logger::log::Logger;

use relcomm::reliable_communication::ReliableCommunication;
use relcomm::node::Node;
use logger::debug;
use crate::hashmap::DistrHash;
use crate::config::INTERVAL;

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

    pub fn run(&self) -> (u32, u32) {
        let mut w = 0;
        let mut r = 0;
        // let file = file.as_bytes();
        // remaing_msgs is a random number between 0 and 100
        let mut remaing_msgs = rand::random::<u32>() % 100;
        loop {
            let (msg, key) = if remaing_msgs > 0 {
                let key = rand::random::<u32>();
                (self.get_rnd_msg(10), key)
            } else {
                (vec![0; 10], self.id as u32)
            };
            self.hash_table.write(&key, msg);
            remaing_msgs -= 1;
            w += 1;
            thread::sleep(Duration::from_millis(INTERVAL));
            let msg = self.hash_table.read(&key);
            match msg {
                Some(m) => {
                    r += 1;
                    // if the message is all zeros, must exit
                    if m.iter().all(|&x| x == 0) {
                        debug!("Agent {id} finished", id=self.id);
                        break;
                    }
                }
                None => {
                    debug!("Read failed on key {key}");
                }
            }
        }
        return (w, r);
    }

    fn get_rnd_msg(&self, size: usize) -> Vec<u8> {
        let mut msg = Vec::new();
        for _ in 0..size {
            msg.push(rand::random::<u8>());
        }
        msg
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
    debug!("Agent {} created", id);

    let agent = Agent::new(id, nodes, logger)?;
    Ok(agent)
}
