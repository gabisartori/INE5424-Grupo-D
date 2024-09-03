use std::net::{UdpSocket, SocketAddr};
use std::thread;
use std::sync::Arc;
use std::time::Duration;

const TIMEOUT: u64 = 2; // Timeout em segundos para o ACK
const MAX_RETRIES: u8 = 5; // Número máximo de tentativas de retransmissão

fn send(socket: &UdpSocket, addr: &SocketAddr, msg: &[u8]) -> Result<(), String> {
    for _ in 0..MAX_RETRIES {
        socket.send_to(msg, addr).expect("Falha ao enviar mensagem");
        println!("Mensagem enviada: {:?}", msg);

        let mut buf = [0; 1024];
        match socket.recv_from(&mut buf) {
            Ok((amt, _)) => {
                let received = &buf[..amt];
                if received == b"ACK" {
                    println!("ACK recebido!");
                    return Ok(());
                }
            }
            Err(_) => {
                println!("Timeout esperando ACK, retransmitindo...");
            }
        }
    }
    Err("Número máximo de tentativas de envio atingido".into())
}

fn receive(socket: &UdpSocket) -> Result<(), String> {
    let mut buf = [0; 1024];
    match socket.recv_from(&mut buf) {
        Ok((amt, src)) => {
            let msg = &buf[..amt];
            println!("Mensagem recebida de {}: {:?}", src, msg);

            socket.send_to(b"ACK", src).expect("Falha ao enviar ACK");
            println!("ACK enviado para {}", src);

            Ok(())
        }
        Err(e) => Err(format!("Erro ao receber mensagem: {}", e)),
    }
}

fn main() {
    let local_addr1 = "127.0.0.1:3000";
    let local_addr2 = "127.0.0.1:3001";

    let remote_addr1: SocketAddr = local_addr1.parse().expect("Endereço inválido");
    let remote_addr2: SocketAddr = local_addr2.parse().expect("Endereço inválido");

    let socket1 = Arc::new(UdpSocket::bind(local_addr1).expect("Não foi possível bindar o socket1"));
    let socket2 = Arc::new(UdpSocket::bind(local_addr2).expect("Não foi possível bindar o socket2"));

    socket1.set_read_timeout(Some(Duration::new(TIMEOUT, 0))).expect("Erro ao definir timeout para socket1");
    socket2.set_read_timeout(Some(Duration::new(TIMEOUT, 0))).expect("Erro ao definir timeout para socket2");

    let socket1_clone = Arc::clone(&socket1);
    let handle1 = thread::spawn(move || {
        send(&socket1_clone, &remote_addr2, b"Hello from process 1!").expect("Falha ao enviar mensagem do processo 1");
        receive(&socket1_clone).expect("Falha ao receber mensagem no processo 1");
    });

    let socket2_clone = Arc::clone(&socket2);
    let handle2 = thread::spawn(move || {
        receive(&socket2_clone).expect("Falha ao receber mensagem no processo 2");
        send(&socket2_clone, &remote_addr1, b"Hello from process 2!").expect("Falha ao enviar mensagem do processo 2");
    });

    handle1.join().expect("Thread 1 panicked");
    handle2.join().expect("Thread 2 panicked");

    println!("Comunicação entre os processos finalizada.");
}
