use std::time::Duration;
pub const BROADCAST: &str = "AB";
pub const TIMEOUT: Duration = Duration::from_millis(5);
pub const TIMEOUT_LIMIT: u32 = 10;
pub const MESSAGE_TIMEOUT: Duration = Duration::from_millis(1000);
pub const BROADCAST_TIMEOUT: Duration = Duration::from_millis(500);
pub const GOSSIP_RATE: usize = 3;
pub const W_SIZE: usize = 5;
pub const LOSS_RATE: f32= 0.01;
