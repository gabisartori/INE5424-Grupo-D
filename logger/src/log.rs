#[macro_export]
macro_rules! debug_println {
    // This pattern accepts format arguments like println!
    ($($arg:tt)*) => {
        let path = format!("tests/debug.txt");
        let mut file: std::fs::File = match std::fs::OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open(path) {
            Ok(f) => f,
            Err(e) => panic!("Erro ao abrir o arquivo: {}", e)
        };
        let msf = format!("----------\n{}\n----------\n", format!($($arg)*));
        std::io::Write::write_all(&mut file, msf.as_bytes()).expect("Erro ao escrever no arquivo");
    };
}

#[macro_export]
macro_rules! debug_file {
    ($file_path:expr, $msg:expr) => {
        let mut file: std::fs::File = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open($file_path)
        {
            Ok(f) => f,
            Err(e) => panic!("Erro ao abrir o arquivo: {}", e),
        };
        // connverts the message to a string
        // Implements Logger for creating a debug log file
        std::io::Write::write_all(&mut file, $msg).expect("Erro ao escrever no arquivo");
    };
}
#[allow(unused_imports)]
#[allow(unused_variables)]
#[allow(unused_mut)]
use std::{
    fs::File,
    sync::{Arc, Mutex},
};

// // Define states for the agents
// #[derive(Debug, Clone, Copy)]
// pub enum AgentStatus {
//     Initializing,
//     InitFailed,
//     WaitingForPacket,
//     Fragmenting,
//     Sending,
//     Receiving,
//     Completed,
//     FailedToReceive,
//     FailedToSend,
//     Down,
//     Unknown,
// }

#[derive(Debug, Clone, Copy)]
pub enum PacketStatus {
    Sent,
    SentFailed,
    SentAck,
    Received,
    ReceivedAck,
    ReceivedFailed,
    Waiting,
    LastPacket,
    Fragmenting,
    Timeout,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageStatus {
    Sent,
    Received,
    SentFailed,
    ReceivedFailed,
    Waiting,
    Timeout,
    Unknown,
}

// Defines sender type functions
#[derive(Debug, Clone, Copy)]
pub enum SenderType {
    Send,
    SendNonblocking,
    Broadcast,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum LoggerState {
    // AgentState {
    //     state: AgentStatus,
    //     agent_id: u16,
    // },
    PacketSender {
        state: PacketStatus,
        current_agent_id: usize,
        target_agent_id: usize,
        seq_num: usize,
        action: PacketStatus,
    },

    PacketReceiver {
        state: PacketStatus,
        current_agent_id: usize,
        target_agent_id: usize,
        seq_num: usize,
        action: PacketStatus,
    },

    PacketBroadcast {
        state: PacketStatus,
        current_agent_id: usize,
        seq_num: usize,
        action: PacketStatus,
        algorithm: String,
    },

    ReceivedLastPacket {
        state: PacketStatus,
        current_agent_id: usize,
        target_agent_id: usize,
        seq_num: usize,
        action: PacketStatus,
        algorithm: String,
    },

    SentLastPacket {
        state: PacketStatus,
        current_agent_id: usize,
        target_agent_id: usize,
        seq_num: usize,
        action: PacketStatus,
        algorithm: String,
    },

    SentAck {
        state: PacketStatus,
        current_agent_id: usize,
        target_agent_id: usize,
        seq_num: usize,
        action: PacketStatus,
    },

    ReceivedAck {
        state: PacketStatus,
        current_agent_id: usize,
        target_agent_id: usize,
        seq_num: usize,
        action: PacketStatus,
    },

    MessageReceiver {
        state: MessageStatus,
        current_agent_id: usize,
        target_agent_id: usize,
        message_id: u32,
        action: MessageStatus,
    },

    MessageSender {
        state: MessageStatus,
        current_agent_id: usize,
        target_agent_id: usize,
        message_id: u32,
        action: MessageStatus,
    },

    MessageBroadcast {
        state: MessageStatus,
        current_agent_id: usize,
        message_id: Option<u32>,
        action: MessageStatus,
        algorithm: String,
    },
    LogInfo {
        current_agent_id: Option<usize>,
        description: String,
    },
    LogFail {
        current_agent_id: Option<usize>,
        description: String,
    },
    LogWarning {
        current_agent_id: Option<usize>,
        description: String,
    },
}

/// Defines the structure for each log message, which will be used to create the log file.
/// Uses a log_type (LoggerState) to create the log message.
#[derive(Debug, Clone, Copy)]
pub struct DebugLog {}

impl DebugLog {
    pub fn new() -> Self {
        Self {}
    }

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    pub fn get_log(&self, log_type: LoggerState) -> String {
        match log_type {
            // LoggerState::AgentState { state, agent_id } => {
            //     format!("Agent {} , state: {:?}", agent_id, state)
            // }
            LoggerState::PacketSender {
                state,
                current_agent_id,
                target_agent_id,
                seq_num,
                action,
            } => {
                format!(
                    // "Agent {} , state: {:?}, sending packet {} to Agent {}. Next action : {:?}",
                    // current_agent_id, state, seq_num, target_agent_id, action
                    "state: {:?}, sending packet {} to Agent {}.",
                    state, seq_num, target_agent_id
                )
            }

            LoggerState::PacketReceiver {
                state,
                current_agent_id,
                target_agent_id,
                seq_num,
                action,
            } => {
                format!(
                    // "Agent {} , state: {:?}, receiving packet {} from Agent {}. Next action : {:?}",
                    // current_agent_id, state, seq_num, target_agent_id, action
                    "state: {:?}, receiving packet {} from Agent {}.",
                    state, seq_num, target_agent_id,
                )
            }

            LoggerState::PacketBroadcast {
                state,
                current_agent_id,
                seq_num,
                action,
                algorithm,
            } => {
                format!(
                    // "Agent {}, state: {:?}, broadcasting packet {}, type {} . Next action : {:?}",
                    // current_agent_id, state, seq_num, algorithm, action
                    "state: {:?}, broadcasting packet {}, algorithm {}",
                    state, seq_num, algorithm
                )
            }

            LoggerState::SentLastPacket {
                state,
                current_agent_id,
                target_agent_id,
                seq_num,
                action,
                algorithm,
            } => {
                format!(
                    // "Agent {}, state: {:?}, sent last packet ({}), type {} . Next action : {:?}",
                    // current_agent_id, state, seq_num, algorithm, action
                    "state: {:?}, sent last packet ({}), algorithm {}",
                    state, seq_num, algorithm
                )
            }

            LoggerState::ReceivedLastPacket {
                state,
                current_agent_id,
                target_agent_id,
                seq_num,
                action,
                algorithm,
            } => {
                format!(
                    // "Agent {}, state: {:?}, received last packet ({}), type {} . Next action : {:?}",
                    // current_agent_id, state, seq_num, algorithm, action
                    "state: {:?}, received last packet ({}), algorithm {}",
                    state, seq_num, algorithm
                )
            }

            LoggerState::SentAck {
                state,
                current_agent_id,
                target_agent_id,
                seq_num,
                action,
            } => {
                format!(
                    "state: {:?}, sending ack {} to Agent {}.",
                    state, seq_num, target_agent_id
                )
            }

            LoggerState::ReceivedAck {
                state,
                current_agent_id,
                target_agent_id,
                seq_num,
                action,
            } => {
                format!(
                    "state: {:?}, received ack {} from Agent {}.",
                    state, seq_num, target_agent_id
                )
            }

            LoggerState::MessageSender {
                state,
                current_agent_id,
                target_agent_id,
                message_id,
                action,
            } => {
                format!(
                    // "Agent {} , state: {:?}, sending message {} to Agent {}. Next action : {:?}",
                    // current_agent_id, state, message_id, target_agent_id, action
                    "state: {:?}, sending message {} to Agent {}.",
                    state, message_id, target_agent_id
                )
            }

            LoggerState::MessageReceiver {
                state,
                current_agent_id,
                target_agent_id,
                message_id,
                action,
            } => {
                format!(
                    // "Agent {}, state: {:?}, receiving message {} from Agent {}. Next action : {:?}",
                    // current_agent_id, state, message_id, target_agent_id, action
                    "state: {:?}, receiving message {} from Agent {}.",
                    state, message_id, target_agent_id
                )
            }

            LoggerState::MessageBroadcast {
                state,
                current_agent_id,
                message_id,
                action,
                algorithm,
            } => {
                format!(
                    // "Agent {}, state: {:?}, broadcasting message {:?}, type {} . Next action : {:?}",
                    // current_agent_id, state, message_id, algorithm, action
                    "state: {:?}, broadcasting message {:?}, algorithm {} .",
                    state, message_id, algorithm
                )
            }
            LoggerState::LogInfo {
                current_agent_id,
                description,
            } => {
                format!("(Agente {:?}) INFO: {}", current_agent_id, description)
            }
            LoggerState::LogFail {
                current_agent_id,
                description,
            } => {
                format!("(Agente {:?}) FAIL: {}", current_agent_id, description)
            }
            LoggerState::LogWarning {
                current_agent_id,
                description,
            } => {
                format!("(Agente {:?}) WARNING: {}", current_agent_id, description)
            }
        }
    }
}

pub type SharedLogger = Arc<Mutex<Logger>>;

/// Creates the log files for each Agent, and writes the log messages obtained from the DebugLog struct.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Logger {
    debug_level: u8,
    n_agents: usize,
}

#[allow(unused_mut)]
impl Logger {
    pub fn new(debug_level: u8, n_agents: usize) -> Self {
        let mut logger = Self {
            debug_level,
            n_agents,
        };

        logger
    }

    pub fn fail(&mut self, msg: String, agent_num: Option<usize>) {
        let logger_state = LoggerState::LogFail {
            current_agent_id: agent_num,
            description: msg,
        };
        self.log(logger_state);
    }

    pub fn warning(&mut self, msg: String, agent_num: Option<usize>) {
        let logger_state = LoggerState::LogWarning {
            current_agent_id: agent_num,
            description: msg,
        };
        self.log(logger_state);
    }

    pub fn info(&mut self, msg: String, agent_num: Option<usize>) {
        let logger_state = LoggerState::LogInfo {
            current_agent_id: agent_num,
            description: msg,
        };
        self.log(logger_state);
    }

    pub fn log(&mut self, logger_state: LoggerState) {
        // with the logger state, we can get the log message
        let log = DebugLog::new().get_log(logger_state.clone());
        let msg_buffer = format!("{}\n", log);

        if self.debug_level > 0 {
            // write to the log file
            let agent_id = match logger_state.clone() {
                LoggerState::PacketSender {
                    current_agent_id, ..
                }
                | LoggerState::PacketReceiver {
                    current_agent_id, ..
                }
                | LoggerState::PacketBroadcast {
                    current_agent_id, ..
                }
                | LoggerState::SentLastPacket {
                    current_agent_id, ..
                }
                | LoggerState::ReceivedLastPacket {
                    current_agent_id, ..
                }
                | LoggerState::SentAck {
                    current_agent_id, ..
                }
                | LoggerState::ReceivedAck {
                    current_agent_id, ..
                }
                | LoggerState::MessageSender {
                    current_agent_id, ..
                }
                | LoggerState::MessageReceiver {
                    current_agent_id, ..
                } => current_agent_id,
                LoggerState::MessageBroadcast {
                    current_agent_id, ..
                } => current_agent_id,
                LoggerState::LogFail {
                    current_agent_id, ..
                } => current_agent_id.unwrap_or(usize::MAX),
                LoggerState::LogInfo {
                    current_agent_id, ..
                } => current_agent_id.unwrap_or(usize::MAX),
                LoggerState::LogWarning {
                    current_agent_id, ..
                } => current_agent_id.unwrap_or(usize::MAX),
            };

            if agent_id == usize::MAX {
                let path = format!("src/log/log_msgs.txt");
                debug_file!(path, msg_buffer.as_bytes());
            } else {
                // write to the agent log file
                let path = format!("src/log/agent_{}.txt", agent_id);
                debug_file!(path, msg_buffer.as_bytes());
            }
        }
    }
}

// TODO: Implement timeout controller, improve log types
