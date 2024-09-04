mod lib {pub mod reliable_communication;}
use lib::reliable_communication;

use std::net::UdpSocket;
use std::thread;
use std::sync::mpsc::{self, RecvTimeoutError};

fn client_1() {
    reliable_communication::send("Hello world");
    std::thread::sleep(std::time::Duration::from_secs(1));
    reliable_communication::send("Hello world");
}

fn client_2() {
    let socket = UdpSocket::bind("localhost:3001").expect("Could not bind to address");
    std::thread::sleep(std::time::Duration::from_secs(3));
    let _ = socket.send_to("Hello world".as_bytes(), "localhost:3000").expect("Couldn't send data");
    std::thread::sleep(std::time::Duration::from_secs(3));
    let _ = socket.send_to("Hello again".as_bytes(), "localhost:3000").expect("Couldn't send data");

    println!("Client thread has finished executing");
}

fn server(tx: mpsc::Sender<()>) {
    let socket = UdpSocket::bind("localhost:3000").expect("Could not bind to address");
    println!("Listening on {}", socket.local_addr().unwrap());
    
    let mut buf = [0; 20];
    loop {
        let (number_of_bytes, src_addr) = socket.recv_from(&mut buf).expect("Didn't receive data");
        let received = std::str::from_utf8(&buf[..number_of_bytes]).expect("Couldn't convert data to string");
        if tx.send(()).is_err() {
            break;
        }
        println!("Received message from {src_addr}: {received}");
    }
    println!("Server thread has finished executing");
}

fn main() {
    let (tx, rx) = std::sync::mpsc::channel();

    let client = thread::spawn(client_2);
    let server = thread::spawn(|| server(tx));
    let timeout = std::time::Duration::from_secs(10);
    loop {
        match rx.recv_timeout(timeout) {
            Ok(()) => println!("Server received a message"),
            Err(RecvTimeoutError::Timeout) => {
                println!("Server was up for {} seconds", timeout.as_secs());
                break;
            },
            Err(RecvTimeoutError::Disconnected) => {println!("Client thread disconnected"); break;},
        }
    }
}
