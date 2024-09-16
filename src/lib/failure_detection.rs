/*
A camada de detecção de defeitos (Failure Detection) permite que processos
    participantes na comunicação monitorem uns aos outros
e sinalizem os protocolos de comunicação confiável sobre possíveis saídas de processos do grupo
(intencionais ou não-intencionais, no caso de falhas).
*/

use std::net::SocketAddr;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::{TIMEOUT, HEARTBEAT_INTERVAL, FAILURE_DETECTION_INTERVAL};

// Estrutura para armazenar informações sobre os processos
pub struct Process {
    addr: SocketAddr,
    last_heartbeat: Instant,
    is_alive: bool,
}

// Estrutura para a camada de detecção de defeitos
pub struct FailureDetection {
    // um hashmap de processos, usando o endereço como chave
    processes: HashMap<SocketAddr, Process>,
}

impl FailureDetection {
    // Função para inicializar a camada de detecção de defeitos
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    // Função para adicionar um novo processo ao grupo
    pub fn add_process(&mut self, addr: SocketAddr) {
        self.processes.insert(addr, Process {
            addr,
            last_heartbeat: Instant::now(),
            is_alive: true,
        });
    }

    // Função para remover um processo do grupo
    pub fn remove_process(&mut self, addr: &SocketAddr) {
        self.processes.remove(addr);
    }

    // Função para atualizar o timestamp do último heartbeat de um processo
    pub fn update_heartbeat(&mut self, addr: &SocketAddr) {
        if let Some(process) = self.processes.get_mut(addr) {
            process.last_heartbeat = Instant::now();
        }
    }

    // Função para verificar se um processo está vivo
    pub fn is_alive(&self, addr: &SocketAddr) -> bool {
        if let Some(process) = self.processes.get(addr) {
            process.is_alive
        } else {
            false
        }
    }

    // Função para verificar se um processo está morto
    pub fn is_dead(&self, addr: &SocketAddr) -> bool {
        if let Some(process) = self.processes.get(addr) {
            process.last_heartbeat.elapsed() > Duration::from_millis(TIMEOUT as u64)
        } else {
            false
        }
    }

    // Função para verificar se um processo está morto
    pub fn detect_failures(&mut self) {
        for (addr, process) in self.processes.iter_mut() {
            if process.last_heartbeat.elapsed() > Duration::from_millis(TIMEOUT as u64) {
                process.is_alive = false;
            }
        }
    }

    // Função para enviar um heartbeat para um processo
    pub fn send_heartbeat(&self, addr: &SocketAddr) {
        // Implementação da lógica de envio de heartbeat
        // Exemplo: self.communication.send(addr, "HEARTBEAT");
    }

    // Função para receber um heartbeat de um processo
    pub fn receive_heartbeat(&mut self, addr: &SocketAddr) {
        // Implementação da lógica de recebimento de heartbeat
        // Exemplo: self.communication.receive(addr, "HEARTBEAT");
        self.update_heartbeat(addr);
    }
}
