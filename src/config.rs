use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::clone::Clone;

#[derive(Clone)]
pub struct Node {
    pub addr: SocketAddr,
    pub agent_number: u32
}

// Endereço IP do localhost
pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const MAYKON: IpAddr = IpAddr::V4(Ipv4Addr::new(150, 162, 77, 208));
pub const UFSC: IpAddr = IpAddr::V6(Ipv6Addr::new(0x2801, 0x84, 0, 0x2, 0, 0, 0, 0x10));
pub const EU_UFSC: IpAddr = IpAddr::V4(Ipv4Addr::new(150, 162, 77, 181));

pub const P1_ADDR: Node = Node{addr: SocketAddr::new(MAYKON, 3000), agent_number: 1};
pub const P2_ADDR: Node = Node{addr: SocketAddr::new(LOCALHOST, 3001), agent_number: 1};
pub const P3_ADDR: Node = Node{addr: SocketAddr::new(LOCALHOST, 3002), agent_number: 2};
pub const TIMEOUT: u64 = 1000; // 1 segundo
pub const HEARTBEAT_INTERVAL: u64 = 500; // 500 milissegundos
pub const FAILURE_DETECTION_INTERVAL: u64 = 1000; // 1 segundo
pub const BUFFER_SIZE: usize = 1024;

// um vetor com os endereços dos processos
pub const NODES: [Node; 1] = [P1_ADDR];
