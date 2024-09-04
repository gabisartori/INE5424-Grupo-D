mod lib {pub mod reliable_communication;}
use lib::reliable_communication;

use std::net::UdpSocket;

fn client_1() {
    reliable_communication::send("Hello world");
    std::thread::sleep(std::time::Duration::from_secs(1));
    reliable_communication::send("Hello world");
}

fn client_2() {
    let socket = UdpSocket::bind("localhost:3001").expect("Could not bind to address");
    let _ = socket.send_to("Hello world".as_bytes(), "localhost:3000").expect("Couldn't send data");
}

fn server() {
    let socket = UdpSocket::bind("localhost:3000").expect("Could not bind to address");
    println!("Listening on {}", socket.local_addr().unwrap());
    let mut buf = [0; 10];
    loop {
        let (number_of_bytes, src_addr) = socket.recv_from(&mut buf).expect("Didn't receive data");
        let received = std::str::from_utf8(&buf[..number_of_bytes]).expect("Couldn't convert data to string");
        println!("Received message: {}", received);
    }
}

fn main() {
    client_1();
    server();
}
