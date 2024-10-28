#[derive(PartialEq)]
pub enum Action {
    Send {
        destination: usize,
        message: String
    },
    Receive {
        message: String
    },
    Broadcast {
        message: String
    },
    Die {
        after_n_messages: u32
    }
}

const MSG_0: &str = "
--------------------------------------
START OF MESSAGE 0
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

const MSG_1: &str = "
--------------------------------------
START OF MESSAGE 1
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

const MSG_2: &str = "
--------------------------------------
START OF MESSAGE 2
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

type Test = Vec<Vec<Action>>;

#[allow(dead_code)]
pub fn send_test_1() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Send {
                destination: 1,
                message: "message_0".to_string(),
            }
        ],
        // Agent 1
        vec![
            Action::Receive {
                message: "message_0".to_string(),
            }
        ]
    ]
}

#[allow(dead_code)]
pub fn send_test_2() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Send {
                destination: 1,
                message: "message_0".to_string(),
            }
        ],
        // Agent 1
        vec![
            Action::Send { destination: 2, message: MSG_0.to_string() },
            Action::Receive {
                message: "message_0".to_string(),
            }
        ],
        // Agent 2
        vec![
            Action::Receive {
                message: MSG_0.to_string(),
            }
        ]
    ] 
}

#[allow(dead_code)]
pub fn broadcast_test_1() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Broadcast {
                message: "message_0".to_string(),
            },
            Action::Receive { message: "message_0".to_string() },
        ],
        // Agent 1
        vec![
            Action::Receive {
                message: "message_0".to_string(),
            }
        ],
        // Agent 2
        vec![
            Action::Receive {
                message: "message_0".to_string(),
            }
        ]
    ]
}

#[allow(dead_code)]
pub fn broadcast_test_2() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Broadcast {
                message: MSG_0.to_string(),
            },
            Action::Receive { message: MSG_0.to_string() },
        ],
        // Agent 1
        vec![
            Action::Receive {
                message: MSG_0.to_string(),
            }
        ],
        // Agent 2
        vec![
            Action::Receive {
                message: MSG_0.to_string(),
            }
        ]
    ]
}

#[allow(dead_code)]
pub fn broadcast_test_3() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Receive { message: "Mensagem 0".to_string() },
            Action::Die { after_n_messages: 1 },
            Action::Receive { message: "Mensagem 1".to_string() },
            Action::Receive { message: "Mensagem 2".to_string() },

        ],
        // Agent 1
        vec![
            Action::Receive { message: "Mensagem 0".to_string() },
            Action::Receive { message: "Mensagem 1".to_string() },
            Action::Receive { message: "Mensagem 2".to_string() },
        ],
        // Agent 2
        vec![
            Action::Broadcast { message: "Mensagem 0".to_string() },
            Action::Broadcast { message: "Mensagem 1".to_string() },

            Action::Receive { message: "Mensagem 0".to_string() },
            Action::Receive { message: "Mensagem 1".to_string() },
            Action::Receive { message: "Mensagem 2".to_string() },

        ],
        // Agent 3
        vec![
            Action::Receive { message: "Mensagem 0".to_string() },
            Action::Receive { message: "Mensagem 1".to_string() },
            Action::Broadcast { message: "Mensagem 2".to_string() },
            Action::Receive { message: "Mensagem 2".to_string() },
        ]
    ]
}

#[allow(dead_code)]
pub fn broadcast_test_4() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Receive { message: MSG_0.to_string() },
            Action::Die { after_n_messages: 1 },
            Action::Receive { message: MSG_1.to_string() },
            Action::Receive { message: MSG_2.to_string() },

        ],
        // Agent 1
        vec![
            Action::Receive { message: MSG_0.to_string() },
            Action::Receive { message: MSG_1.to_string() },
            Action::Receive { message: MSG_2.to_string() },
        ],
        // Agent 2
        vec![
            Action::Broadcast { message: MSG_0.to_string() },
            Action::Broadcast { message: MSG_1.to_string() },

            Action::Receive { message: MSG_0.to_string() },
            Action::Receive { message: MSG_1.to_string() },
            Action::Receive { message: MSG_2.to_string() },

        ],
        // Agent 3
        vec![
            Action::Receive { message: MSG_0.to_string() },
            Action::Receive { message: MSG_1.to_string() },
            Action::Broadcast { message: MSG_2.to_string() },
            Action::Receive { message: MSG_2.to_string() },
        ]
    ]
}