use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::clone::Clone;

#[derive(Clone)]
pub struct Node {
    pub addr: SocketAddr,
    pub agent_number: u32
}

// Endereços IP úteis
pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const MAYKON: IpAddr = IpAddr::V4(Ipv4Addr::new(150, 162, 77, 208));
pub const SARTORI: IpAddr = IpAddr::V4(Ipv4Addr::new(150, 162, 77, 181));


/*
    Existem dois estados principais para os valores de AGENT_NUM e NODES
    1. Cenário de teste local: AGENT_NUM é um valor qualquer e não há NODES remotos
    2. Cenário que imita um ambiente distribuído: AGENT_NUM é 1 (apenas a própria máquina) e NODES é um vetor com os vários endereços de outras máquinas pertencentes ao grupo
*/

// Quantia de agentes locais a serem criados
pub const AGENT_NUM: u32 = 2;
pub const N_MSGS: u32 = 3;

// Endereços de agentes externos
// pub const NODES: Option<&[Node]> = Some(&[
//     Node { addr: SocketAddr::new(MAYKON, 8080), agent_number: 1 },
//     Node { addr: SocketAddr::new(MAYKON, 8081), agent_number: 2 },
//     Node { addr: SocketAddr::new(MAYKON, 8082), agent_number: 3 },
//     Node { addr: SocketAddr::new(MAYKON, 8083), agent_number: 4 }
// ]);
pub const NODES: Option<&[Node]> = None;


// Configurações da comunicação
pub const W_SIZE: usize = 5;
pub const TIMEOUT: u64 = 100; // 0.1 segundo
pub const HEARTBEAT_INTERVAL: u64 = 500; // 500 milissegundos
pub const FAILURE_DETECTION_INTERVAL: u64 = 1000; // 1 segundo
pub const BUFFER_SIZE: usize = 1024;

// Mensagem com 10240 bytes para testes
pub const LARGE_MSG: &str = "
--------------------------------------
START OF MESSAGE
--------------------------------------
Universidade Federal de Santa Catarina - UFSC
Departamento em Informática e Estatística
INE5424-06208B - Sistemas Operacionais II
Semestre 2024/2
Projeto e Implementação de uma Biblioteca para Comunicação Confiável entre Processos
1. Objetivos e Escopo
Este projeto consiste em desenvolver uma biblioteca de comunicação capaz de garantir a
entrega confiável de mensagens entre os processos participantes de um grupo. Dessa
forma, programas que utilizem a biblioteca irão usufruir de garantias na entrega de
mensagens, como entrega confiável, difusão com entrega para todos os processos corretos
ou nenhum, ou, ainda, garantias de ordem na entrega, como ordenação FIFO e total.
A biblioteca deverá disponibilizar para o usuário as primitivas send(id,m) e receive(m)
para mensagens destinadas a um processo específico (comunicação 1:1), onde id é o
identificador do destinatário e m é uma mensagem; e primitivas broadcast(m) e
deliver(m) para mensagens destinadas a todos os participantes (comunicação 1:n),
sendo m uma mensagem e n o número total de participantes.
O desafio está em preservar propriedades de entrega confiável em ambientes não
confiáveis, onde processos podem falhar e mensagens podem ser perdidas quando
transmitidas pelos protocolos de rede subjacentes. A biblioteca deve ser implementada na
linguagem C++ e a comunicação entre processos deve ser feita por sockets padrão
(Berkeley sockets / POSIX.1-2008). Não é admitido o uso de bibliotecas pré-existentes para
comunicação confiável ou que apresentem outras abstrações de comunicação para além do
uso de sockets padrão.
--------------------
END OF MESSAGE
--------------------
";
