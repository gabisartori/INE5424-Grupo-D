pub const P1_ADDR: &str = "127.0.0.1:3000";
pub const P2_ADDR: &str = "127.0.0.1:3001";
pub const P3_ADDR: &str = "127.0.0.1:3002";

pub const TIMEOUT: u64 = 1000; // 1 segundo
pub const HEARTBEAT_INTERVAL: u64 = 500; // 500 milissegundos
pub const FAILURE_DETECTION_INTERVAL: u64 = 1000; // 1 segundo
pub const BUFFER_SIZE: usize = 1024;

// um vetor com os endereços dos processos
// pub const NODES: [&str; 2] = [P1_ADDR, P2_ADDR, P3_ADDR];
