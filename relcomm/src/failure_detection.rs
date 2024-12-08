/*
A camada de detecção de defeitos (Failure Detection) permite que processos
    participantes na comunicação monitorem uns aos outros
e sinalizem os protocolos de comunicação confiável sobre possíveis saídas de processos do grupo
(intencionais ou não-intencionais, no caso de falhas).
*/
use std::{thread, vec};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, MutexGuard};
use logger::debug;

use crate::config::{HEARTBEAT_INTERVAL, HEARTBEAT_MISS_LIMIT};
use crate::node::{Node, NodeState};
use crate::channels::Channel;
use crate::packet::Packet;

pub struct FailureDetection {
    group: Arc<Mutex<Vec<Node>>>,
    hb_miss_cnt: Vec<i32>,
}

impl FailureDetection {
    /// Creates a new FailureDetection instance
    pub fn new(group: Arc<Mutex<Vec<Node>>>) -> Self {
        let agent_num = group.lock().expect("Failed to lock group on new").len();
        Self {
            group,
            hb_miss_cnt: vec![-1; agent_num],
        }
    }

    pub fn get_hbs(group: &Arc<Mutex<Vec<Node>>>, host: &Node) -> (Vec<Packet>, usize) {
        let group = group
            .lock()
            .expect("Failed to lock group on get_hbs");
        let len = group.len();
        if len == 0 {
            panic!("No agents in the group");
        }
        let heart_beats = group.iter().map(|node| {
            Packet::heart_beat(&host, node.addr)
        }).collect::<Vec<Packet>>();
        (heart_beats, len)
    }

    /// Starts the failure detection process
    /// This function will run in a separate thread
    /// It will send heartbeats to all nodes in the group
    /// wait for heartbeats from all nodes in the group
    /// and mark nodes as suspect or dead if they don't respond
    pub fn run(&mut self, hb_rx: Receiver<Packet>, channel: Arc<Channel>,
        heart_beats: Vec<Packet>, agent_num: usize) {
        loop {
            // Broadcast heartbeats
            for pkt in heart_beats.iter() {
                channel.send(pkt);
            }
            thread::sleep(HEARTBEAT_INTERVAL);
            let mut group = self.group
                .lock()
                .expect("Failed to lock group on failure_detection loop");
            let hb_miss = Self::process_heartbeats(&mut group, &mut self.hb_miss_cnt, &hb_rx);
            for i in 0..agent_num {
                if hb_miss[i] == 1 {
                    self.hb_miss_cnt[i] += 1;
                    if self.hb_miss_cnt[i] >= HEARTBEAT_MISS_LIMIT {
                        if group[i].state != NodeState::Dead {
                            debug!("Agent {} is dead", group[i].agent_number);
                        }
                        group[i].state = NodeState::Dead;
                    } else {
                        group[i].state = NodeState::Suspect;
                    }
                }
            }
        }
    }

    fn process_heartbeats(group: &mut MutexGuard<'_, Vec<Node>>,
        hb_miss_cnt: &mut Vec<i32>, hb_rx: &Receiver<Packet>)
        -> Vec<i32> {
        let mut hb_miss = vec![1; group.len()];
        while let Ok(hb) = hb_rx.try_recv() {
            let id = hb.header.seq_num as usize;
            hb_miss_cnt[id] = 0;
            hb_miss[id] = 0;
        }
        hb_miss
    }

    pub fn handle_hb(hb: &Packet, group_locked: &Arc<Mutex<Vec<Node>>>) {
        let agt_num = hb.header.seq_num as usize;
        let mut group = group_locked
            .lock()
            .expect("Failed to lock group on handle_hb");
        group[agt_num].state = NodeState::Alive;
    }
}
