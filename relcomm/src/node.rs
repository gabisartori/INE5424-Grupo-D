use std::net::SocketAddr;

#[derive(Clone, Debug, PartialEq)]
pub enum NodeState {
    Unborn,
    Alive,
    Dead,
    Suspect,
}

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
            state: NodeState::Unborn,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.state == NodeState::Dead
    }

    pub fn non_initiated(&self) -> bool {
        self.state == NodeState::Unborn
    }

    pub fn is_alive(&self) -> bool {
        self.state == NodeState::Alive
    }
}
