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

    pub fn new(group: Arc<Mutex<Vec<Node>>>,
        channel: Arc<Channel>
        , host: Node) -> Self {
        Self {
            group,
            channel,
            host,
        }
    }

    fn send_heart_beat(&self) {
        let group = self.group.lock().expect("Failed to lock group on send_heart_beat");
        for node in group.iter() {
            let pkt = Packet::heart_beat(&self.host, node.addr);
            self.channel.send(&pkt);
        }
    }
    pub fn run(self, hb_rx: mpsc::Receiver<Packet>) {
        let agent_number = self.group.lock().unwrap().len();
        let mut hb_miss_cnt: Vec<i32> = vec![-1; agent_number];
        loop {
            self.send_heart_beat();
            thread::sleep(HEARTBEAT_INTERVAL);
            let mut group = self.group.lock().expect("Failed to lock group on failure_detection");
            let mut hb_miss = vec![0; agent_number];
            while let Ok(hb) = hb_rx.try_recv() {
                let id = hb.header.seq_num as usize;
                let node = &mut group[id];
                match node.state {
                    NodeState::ALIVE => {}
                    _ => {
                        node.state = NodeState::ALIVE;
                        hb_miss_cnt[id] = 0;
                        hb_miss[id] = 1;
                    }
                }
            }
            for i in 0..agent_number {
                if hb_miss[i] == 0 {
                    hb_miss_cnt[i] += 1;
                    group[i].state = NodeState::SUSPECT;
                    if hb_miss_cnt[i] >= HEARTBEAT_MISS_LIMIT {
                        group[i].state = NodeState::DEAD;
                    }
                }
            }            
        }
    }
}
