use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::clone::Clone;

#[derive(Clone)]
pub struct Node {
    pub addr: SocketAddr,
    pub agent_number: u32
}

// Endereços IP úteis
pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const MAYKON: IpAddr = IpAddr::V4(Ipv4Addr::new(150, 162, 77, 208));
pub const SARTORI: IpAddr = IpAddr::V4(Ipv4Addr::new(150, 162, 77, 181));


/*
    Existem dois estados principais para os valores de AGENT_NUM e NODES
    1. Cenário de teste local: AGENT_NUM é um valor qualquer e não há NODES remotos
    2. Cenário que imita um ambiente distribuído: AGENT_NUM é 1 (apenas a própria máquina) e NODES é um vetor com os vários endereços de outras máquinas pertencentes ao grupo
*/

// Quantia de agentes locais a serem criados
pub const AGENT_NUM: u32 = 4;

// Endereços de agentes externos
// pub const NODES: Option<&[Node]> = Some(&[
//     Node { addr: SocketAddr::new(MAYKON, 8080), agent_number: 1 },
//     Node { addr: SocketAddr::new(MAYKON, 8081), agent_number: 2 },
//     Node { addr: SocketAddr::new(MAYKON, 8082), agent_number: 3 },
//     Node { addr: SocketAddr::new(MAYKON, 8083), agent_number: 4 }
// ]);
pub const NODES: Option<&[Node]> = None;


// Configurações da comunicação
pub const W_SIZE: usize = 5;
pub const TIMEOUT: u128 = 100; // 0.1 segundo
pub const HEARTBEAT_INTERVAL: u64 = 500; // 500 milissegundos
pub const FAILURE_DETECTION_INTERVAL: u64 = 1000; // 1 segundo
pub const BUFFER_SIZE: usize = 1024;
