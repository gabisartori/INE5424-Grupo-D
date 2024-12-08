use std::net::SocketAddr;
use std::fmt::{Debug, Display, Formatter, self};

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

    pub fn set_as_dead(&mut self) {
        self.state = NodeState::Dead;
    }
}

impl Display for NodeState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NodeState::Suspect => write!(f, "Suspect"),
            NodeState::Unborn => write!(f, "Unborn"),
            NodeState::Alive => write!(f, "Alive"),
            NodeState::Dead => write!(f, "Dead"),
        }
    }    
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Agent {} -> {} <-", self.agent_number, self.state)
    }
}
