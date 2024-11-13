use std::net::SocketAddr;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub addr: SocketAddr,
    pub agent_number: usize,
    pub state: NodeState,
}

impl Node {
    pub fn new(addr: SocketAddr, agent_number: usize) -> Self {
        Self {
            addr,
            agent_number,
            state: NodeState::NonInit,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeState {
    NonInit,
    ALIVE,
    DEAD,
    SUSPECT,
}
