/*
A camada de detecção de defeitos (Failure Detection) permite que processos
    participantes na comunicação monitorem uns aos outros
e sinalizem os protocolos de comunicação confiável sobre possíveis saídas de processos do grupo
(intencionais ou não-intencionais, no caso de falhas).
*/
use std::{thread, vec};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use crate::config::{HEARTBEAT_INTERVAL, HEARTBEAT_MISS_LIMIT};
use crate::node::{Node, NodeState};
use crate::channels::Channel;
use crate::packet::Packet;

pub struct FailureDetection {
    group: Arc<Mutex<Vec<Node>>>,
    channel: Arc<Channel>,
    host: Node,
}

impl FailureDetection {
    /// Creates a new FailureDetection instance
    pub fn new(group: Arc<Mutex<Vec<Node>>>,
        channel: Arc<Channel>
        , host: Node) -> Self {
        Self {
            group,
            channel,
            host,
        }
    }
    /// Starts the failure detection process
    /// This function will run in a separate thread
    /// It will send heartbeats to all nodes in the group
    /// wait for heartbeats from all nodes in the group
    /// and mark nodes as suspect or dead if they don't respond
    pub fn run(self, hb_rx: mpsc::Receiver<Packet>) {
        let (heart_beats, agent_num) = {
            let group = self.group.lock().expect("Failed to lock group on failure_detection start");
            let len = group.len();
            if len == 0 {
                panic!("No agents in the group");
            }
            let heart_beats = group.iter().map(|node| {
                Packet::heart_beat(&self.host, node.addr)
            }).collect::<Vec<Packet>>();
            (heart_beats, len)
        };
        let mut hb_miss_cnt: Vec<i32> = vec![-1; agent_num];
        loop {
            // Broadcast heartbeats
            for pkt in heart_beats.iter() {
                self.channel.send(pkt);
            }
            thread::sleep(HEARTBEAT_INTERVAL);
            let mut group = self.group.lock().expect("Failed to lock group on failure_detection");
            let mut hb_miss = vec![1; agent_num];
            while let Ok(hb) = hb_rx.try_recv() {
                let id = hb.header.seq_num as usize;
                hb_miss_cnt[id] = 0;
                hb_miss[id] = 0;
                let node = &mut group[id];
                node.state = NodeState::Alive;
            }
            for i in 0..agent_num {
                if hb_miss[i] == 1 {
                    hb_miss_cnt[i] += 1;
                    if hb_miss_cnt[i] >= HEARTBEAT_MISS_LIMIT {
                        group[i].state = NodeState::Dead;
                    } else {
                        group[i].state = NodeState::Suspect;
                    }
                }
            }            
        }
    }
}
