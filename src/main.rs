mod lib {
    pub mod reliable_communication;
}

use lib::reliable_communication;

fn client_1() {
    reliable_communication::send("Hello world");
    std::thread::sleep(std::time::Duration::from_secs(1));
    reliable_communication::send("Hello world");
}

fn client_2() {
    reliable_communication::send("Hello world");
}

fn server() {
    for i in 0..3 {
        let message = reliable_communication::receive();
        println!("Received message: {}", message);
    }
}

fn main() {
    client_1();
    server();
}
