mod lib {pub mod reliable_communication;}

use std::thread;
use std::sync::Arc;
use std::env;

struct Agent {
    port: u16,
    agent_number: u32,
}

impl Agent {
    fn new(port: u16, agent_number: u32) -> Agent {
        Agent {
            port,
            agent_number,
        }
    }

    fn listener(&self) {
        println!("Agent {} is listening", self.agent_number);

    }

    fn sender(&self, user_controlled: bool) {
        println!("Agent {} is sending messages", self.agent_number);

    }

    fn run(self: Arc<Self>) {
        let listener_clone = Arc::clone(&self);
        let sender_clone = Arc::clone(&self);

        let listener = thread::spawn(move || listener_clone.listener());
        let sender = thread::spawn(move || sender_clone.sender(false));

        listener.join().unwrap();
        sender.join().unwrap();
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();
    let agent_number = match args.len() {
        2 => args[1].parse::<u32>().unwrap(),
        _ => 4,
    };

    let mut agent_handlers: Vec<thread::JoinHandle<()>> = Vec::new();

    let start_port = 3000;
    for i in 0..agent_number {
        let agent = Arc::new(Agent::new(start_port + (i as u16), i));
        agent_handlers.push(thread::spawn(move || agent.run()));
    }

    for handler in agent_handlers {
        handler.join().unwrap();
    }
}
