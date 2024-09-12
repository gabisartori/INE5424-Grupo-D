use std::net::UdpSocket;

pub struct ReliableCommunication {
    socket: UdpSocket,
}

impl ReliableCommunication {
    pub fn new(port: u16) -> ReliableCommunication {
        println!("Creating new ReliableCommunication instance");
        let socket = UdpSocket::bind(format!("127.0.0.1:{}", port)).unwrap();
        println!("Socket bound to {}", socket.local_addr().unwrap());
        ReliableCommunication {
            socket: socket,
        }
    }


    pub fn send(&self, host: &str, port: u16, message: &str) {
        self.socket.send_to(message.as_bytes(), format!("{}:{}", host, port)).unwrap();
    }

    pub fn receive(&self, message: &mut [u8]) -> (usize, std::net::SocketAddr) {
        self.socket.recv_from(message).unwrap()
    }

    pub fn broadcast(message: &str) {
        println!("Broadcasting message: {}", message);
    }

    pub fn deliver(message: &str) {
        println!("Delivering message: {}", message);
    }
}