#![allow(dead_code)]

#[derive(PartialEq, Clone)]
pub enum SendAction {
    Send {
        destination: usize,
        message: String
    },
    Broadcast {
        message: String
    },
    DieAfterSend {}
} 

#[derive(PartialEq, Clone)]
pub enum ReceiveAction {
    Receive {
        message: String
    },
    DieAfterReceive {
        after_n_messages: u32
    }
}

#[derive(PartialEq, Clone)]
pub enum Action {
    Send(SendAction),
    Receive(ReceiveAction),
    Die()
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

/// Um enviador e um recebedor
pub fn send_test_1() -> Test {
    vec![
        // Agent 0
        vec![Action::Send(SendAction::Send { destination: 1, message: MSG_0.to_string() })],
        // Agent 1
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })]
    ]
}

/// 3 Nodos, os dois primeiros enviam mensagens e os dois últimos recebem
pub fn send_test_2() -> Test {
    vec![
        // Agent 0
        vec![Action::Send(SendAction::Send { destination: 1, message: MSG_0.to_string() })],
        // Agent 1
        vec![
            Action::Send(SendAction::Send { destination: 2, message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })
        ],
        // Agent 2
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })]
    ] 
}

/// 3 Nodos, os dois primeiros enviam para o terceiro
pub fn send_test_3() -> Test {
    vec![
        // Agent 0
        vec![Action::Send(SendAction::Send { destination: 2, message: "message_0".to_string() })],
        // Agent 1
        vec![Action::Send(SendAction::Send { destination: 2, message: MSG_0.to_string() })],
        // Agent 2
        vec![
            Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })
        ]
    ]
}

/// Um enviador mas o recebedor morre antes de receber
pub fn send_test_4() -> Test {
    vec![
        // Agent 0
        vec![Action::Send(SendAction::Send { destination: 1, message: "message_0".to_string() })],
        // Agent 1
        vec![Action::Die()]
    ]
}

/// 10 Nodos, um faz broadcast
pub fn broadcast_test_1() -> Test {
    vec![
        // Agent 0
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
        vec![
            Action::Send(SendAction::Broadcast { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })
        ],
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() })],
    ]
}

/// 10 Nodos, um faz broadcast e um nem nasce
pub fn broadcast_test_2() -> Test {
    vec![
        // Agent 0
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        // Agent 1
        vec![Action::Die()],
        // Agent 2
        vec![
            Action::Send(SendAction::Broadcast {  message: "message_0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })
            ],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
    ]
}

/// 10 Nodos, um faz broadcast e o líder nem nasce
pub fn broadcast_test_3() -> Test {
    vec![
        // Agent 0
        vec![Action::Die()],
        // Agent 1
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        // Agent 2
        vec![
            Action::Send(SendAction::Broadcast { message: "message_0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })
            ],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
    ]
}

/// 10 Nodos, um faz broadcast e morre
pub fn broadcast_test_4() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Send(SendAction::Broadcast { message: "message_0".to_string() }),
            Action::Send(SendAction::DieAfterSend {}),
            Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() }),
        ],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
        vec![Action::Receive(ReceiveAction::Receive { message: "message_0".to_string() })],
    ]
}

/// 10 Nodos, um faz vários broadcasts, o líder morre em algum momento
pub fn broadcast_test_5() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Receive(ReceiveAction::DieAfterReceive { after_n_messages: 0 })
            ],
        // Agent 1
        vec![
            Action::Send(SendAction::Broadcast { message: MSG_0.to_string() }),
            Action::Send(SendAction::Broadcast { message: MSG_1.to_string() }),
            Action::Send(SendAction::Broadcast { message: MSG_2.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })

        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: MSG_0.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_1.to_string() }),
            Action::Receive(ReceiveAction::Receive { message: MSG_2.to_string() })    
        ],
    ]
}

/// 10 Nodos, vários fazem broadcast
pub fn broadcast_test_6() -> Test {
    vec![
        // Agent 0
        vec![
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })
        ],
        // Agent 1
        vec![
            Action::Send(SendAction::Broadcast { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })

        ],
        vec![
            Action::Send(SendAction::Broadcast { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })    
        ],
        vec![
            Action::Send(SendAction::Broadcast { message: "2".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })    
        ],
        vec![
            Action::Receive(ReceiveAction::Receive { message: "0".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "1".to_string() }),
            Action::Receive(ReceiveAction::Receive { message: "2".to_string() })    
        ],
    ]
}

// a vec of function pointers to the tests
pub fn all_tests() -> Vec<(&'static str, Test)> {
    vec![
        ("send_1", send_test_1()),
        ("send_2", send_test_2()),
        ("send_3", send_test_3()),
        ("send_4", send_test_4()),
        ("broadcast_1", broadcast_test_1()),
        ("broadcast_2", broadcast_test_2()),
        ("broadcast_3", broadcast_test_3()),
        ("broadcast_4", broadcast_test_4()),
        ("broadcast_5", broadcast_test_5()),
        ("broadcast_6", broadcast_test_6())
    ]
}
