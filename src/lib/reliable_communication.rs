/*
As aplicações de propósito geral acessam a biblioteca a partir da API
disponibilizada pela camada de difusão confiável (Reliable Communication),
permitindo o envio e recebimento de mensagens com garantias de entrega e ordem.
*/

// Importa a camada de canais
use super::channels::Channel;
use crate::config::{BUFFER_SIZE, Node};

use std::net::SocketAddr;

pub struct ReliableCommunication {
    channel: Channel,
    host: SocketAddr,
    group: Vec<Node>,
}

// TODO: Fazer com que a inicialização seja de um grupo

impl ReliableCommunication {
    // Função para inicializar a camada com um canal de comunicação
    pub fn new(host: SocketAddr, group: Vec<Node>) -> Self {
        let channel = Channel::new(&host)
        .expect("\nFalha ao inicializar o canal no nível Rel_Com\n");
        Self { channel: channel, host: host, group: group }
    }

    // Função para enviar mensagem com garantias de comunicação confiável
    pub fn send(&self, dst_addr: &SocketAddr, message: &[u8; BUFFER_SIZE]) {
        /*
        pseudo-código:
        INICIO GoBackN(Sender, Receiver, N)

            base ← 0
            nextSeqNum ← 0
            janela ← N
            bufferACKS ← []

            ENQUANTO houver pacotes a serem enviados OU pacotes aguardando ACK:
                SE nextSeqNum < base + janela E houver pacotes a serem enviados:
                pacote ← PROXIMO_PACOTE()
                ENVIAR pacote[nextSeqNum] PARA Receiver
                nextSeqNum ← nextSeqNum + 1
                FIMSE

                INICIAR temporizador

                ENQUANTO temporizador não expirar:
                SE ACK for recebido E ACK = base OU bufferACKS CONTER ACK:
                    base ← ACK + 1
                    INTERROMPER temporizador
                FIMSE
                FIMENQUANTO

                SE temporizador expirar:
                PARA i DE base ATÉ nextSeqNum - 1:
                    ENVIAR pacote[i] novamente PARA Receiver
                FIMPARA
                REINICIAR temporizador
                FIMSE

            FIMENQUANTO

        FIM GoBackN

        Explicação:
        O remetente mantém uma janela de tamanho N.
        Ele envia pacotes até que o nextSeqNum atinja o limite da janela.
        Se o ACK de um pacote é perdido ou a entrega falha, todos os pacotes a partir do pacote perdido são retransmitidos.
        Quando um ACK é recebido, a janela é movida para frente, permitindo o envio de novos pacotes.
        */
        self.channel.send(&dst_addr, message).expect("Falha ao enviar mensagem no nível Rel_Com\n");
        // Lógica para lidar com confirmação de entrega e retransmissão
    }

    // Função para receber mensagens confiáveis
    pub fn receive(&self, buffer: &mut [u8; BUFFER_SIZE]) -> (usize, SocketAddr) {
        self.channel.receive(buffer).expect("Falha ao receber mensagem no nível Rel_Com\n")

        // Lógica para validar e garantir a entrega confiável
        // self.validate_message(buffer);
        // buffer[..size].to_vec() // Retorna a mensagem recebida
    }

    fn validate_message(&self, message: &[u8; BUFFER_SIZE]) -> bool {
        // Lógica para validar a mensagem recebida
        true
    }
}

