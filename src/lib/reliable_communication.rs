pub fn send(message: &str) {
    println!("Sending message: {}", message);
}

pub fn receive() -> &'static str {
    "Hello, world!"
}

pub fn broadcast(message: &str) {
    println!("Broadcasting message: {}", message);
}

pub fn deliver(message: &str) {
    println!("Delivering message: {}", message);
}
