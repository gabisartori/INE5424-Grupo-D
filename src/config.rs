#![allow(dead_code)]
use std::net::{IpAddr, Ipv4Addr};
pub const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const PORT: u16 = 3000;

#[derive(PartialEq)]
pub enum Broadcast {
    NONE,
    BEB,
    URB,
    AB
}

// Quantia de agentes locais a serem criados
pub const AGENT_NUM: usize = 50;
pub const N_MSGS: u32 = 10;
pub const BUFFER_SIZE: usize = 2<<9;

// Configurações da comunicação
pub const CORRUPTION_RATE: f32 = 0.;
pub const LOSS_RATE: f32 = 0.;
pub const BROADCAST: Broadcast = Broadcast::URB;

pub const HEARTBEAT_INTERVAL: u64 = 500;
pub const FAILURE_DETECTION_INTERVAL: u64 = 1000;

pub const MSG: &str = "
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
