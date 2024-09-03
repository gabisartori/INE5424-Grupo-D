use std::net::{UdpSocket, SocketAddr};
use std::time::Duration;
use std::str;

const TIMEOUT: u64 = 2; // Timeout em segundos para o ACK
const MAX_RETRIES: u8 = 5; // Número máximo de tentativas de retransmissão

fn send(socket: &UdpSocket, addr: &SocketAddr, msg: &[u8]) -> Result<(), String> {
    for _ in 0..MAX_RETRIES {
        // Envia a mensagem
        socket.send_to(msg, addr).expect("Falha ao enviar mensagem");
        println!("Mensagem enviada: {:?}", msg);

        // Aguarda o ACK
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

            // Envia ACK de volta
            socket.send_to(b"ACK", src).expect("Falha ao enviar ACK");
            println!("ACK enviado para {}", src);

            Ok(())
        }
        Err(e) => Err(format!("Erro ao receber mensagem: {}", e)),
    }
}

pub fn run() {
    // Definição dos endereços IP e portas dos processos
    let local_addr = "127.0.0.1:3000";
    let remote_addr: SocketAddr = "127.0.0.1:3001".parse().expect("Endereço inválido");

    // Criação do socket
    let socket = UdpSocket::bind(local_addr).expect("Não foi possível bindar o socket");
    socket.set_read_timeout(Some(Duration::new(TIMEOUT, 0))).expect("Erro ao definir timeout");

    // Envio e recepção de mensagens
    if let Err(e) = send(&socket, &remote_addr, b"Hello, Process 2!") {
        eprintln!("{}", e);
    }

    if let Err(e) = receive(&socket) {
        eprintln!("{}", e);
    }
}
