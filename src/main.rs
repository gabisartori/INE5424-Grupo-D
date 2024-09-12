mod lib {pub mod reliable_communication;}

use std::thread;
use std::sync::Arc;
use std::env;
use rand::Rng;

use lib::reliable_communication;

struct Agent {
    port: u16,
    agent_number: u32,
    communication: reliable_communication::ReliableCommunication
}

impl Agent {
    fn new(port: u16, agent_number: u32) -> Agent {
        Agent {
            port,
            agent_number,
            communication: reliable_communication::ReliableCommunication::new(port)
        }
    }

    fn listener(&self) {
        println!("Agent {} is listening", self.agent_number);
        loop {
            let mut message = [0; 1024];
            let (size, sender) = self.communication.receive(&mut message);
            println!("\nAgent {} receiving {} bytes from {}\nMessage: {}\n", self.agent_number, size, sender, std::str::from_utf8(&message).unwrap());
        }
    }

    fn sender(&self, user_controlled: bool) {
        println!("Agent {} is sending messages", self.agent_number);

        loop {
            let n = rand::thread_rng().gen_range(0..4);
            let host = "127.0.0.1";
            let port = 3000 + n;
            println!("\nAgent {} sending message to {}:{}\n", self.agent_number, host, port);
            let message = format!("Hello agent {}", n);
            self.communication.send(host, port, &message);
            thread::sleep(std::time::Duration::from_secs(rand::thread_rng().gen_range(1..10)));
        }
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

    println!();

    for handler in agent_handlers {
        handler.join().unwrap();
    }
}
