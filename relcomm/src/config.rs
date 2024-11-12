use std::time::Duration;
pub const BROADCAST: &str = "AB";
pub const TIMEOUT: Duration = Duration::from_millis(10);
pub const TIMEOUT_LIMIT: u32 = 100;
pub const MESSAGE_TIMEOUT: Duration = Duration::from_millis(5000);
pub const BROADCAST_TIMEOUT: Duration = Duration::from_millis(10000);
pub const GOSSIP_RATE: usize = 3;
pub const W_SIZE: usize = 5;
pub const LOSS_RATE: f32= 0.01;
