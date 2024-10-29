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
    SentBroadcast,
    SentFailed,
    SentAck,
    SentAckFailed,
    SentLastPacket,
    Received,
    ReceivedBroadcast,
    ReceivedAck,
    ReceivedFailed,
    ReceivedAckFailed,
    ReceivedLastPacket,
    Waiting,
    Timeout,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageStatus {
    Sent,
    SentBroadcast,
    Received,
    ReceivedBroadcast,
    SentFailed,
    ReceivedFailed,
    Waiting,
    Fragmenting,
    Timeout,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum LoggerState {
    // AgentState {
    //     state: AgentStatus,
    //     agent_id: u16,
    // },
    Packet {
        state: PacketStatus,
        current_agent_id: Option<usize>,
        target_agent_id: Option<usize>,
        seq_num: usize,
        // action: PacketAction,
        // algorithm: PacketAlgorithm,
    },
    Message {
        state: MessageStatus,
        current_agent_id: Option<usize>,
        target_agent_id: Option<usize>,
        message_id: usize,
        // action: MessageAction,
        // algorithm: MessageAlgorithm,
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
            LoggerState::Message {
                state,
                current_agent_id,
                target_agent_id,
                message_id,
            } => match state {
                MessageStatus::Sent => {
                    format!(
                        "(State: {:?}) sent message {} to Agent {}",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
                MessageStatus::SentBroadcast => {
                    format!(
                        "(State: {:?}) broadcasted message {} (target: {})",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
                MessageStatus::Received => {
                    format!(
                        "(State: {:?}) received message {} from Agent {}",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
                MessageStatus::ReceivedBroadcast => {
                    format!(
                        "(State: {:?}) received broadcast message {} (leader: {})",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
                MessageStatus::SentFailed => {
                    format!(
                        "(State: {:?}) failed to send message {} to Agent {}",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
                MessageStatus::ReceivedFailed => {
                    format!(
                        "(State: {:?}) failed to receive message {} from Agent {}",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
                MessageStatus::Waiting => {
                    format!(
                        "(State: {:?}) waiting for message {} from Agent {}",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
                MessageStatus::Timeout => {
                    format!(
                        "(State: {:?}) timed out waiting for message {} from Agent {}",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
                MessageStatus::Unknown => {
                    format!(
                        "(State: {:?}) unknown state",
                        state,
                    )
                }
                MessageStatus::Fragmenting => {
                    format!(
                        "(State: {:?}) fragmenting message {} to Agent {}",
                        state,
                        message_id,
                        target_agent_id.unwrap()
                    )
                }
            },
            LoggerState::Packet {
                state,
                current_agent_id,
                target_agent_id,
                seq_num,
            } => match state {
                PacketStatus::Sent => {
                    format!(
                        "(State: {:?}) sending packet {} to Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::SentBroadcast => {
                    format!(
                        "(State: {:?}) broadcasting packet {} (target: {})",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::SentFailed => {
                    format!(
                        "(State: {:?}) failed to send packet {} to Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::SentAck => {
                    format!(
                        "(State: {:?}) sent ACK for packet {} to Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::SentAckFailed => {
                    format!(
                        "(State: {:?}) failed to send ACK for packet {} to Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::SentLastPacket => {
                    format!(
                        "(State: {:?}) sent last packet {} to Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::Received => {
                    format!(
                        "(State: {:?}) received packet {} from Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::ReceivedBroadcast => {
                    format!(
                        "(State: {:?}) received broadcast packet {} (leader: {})",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::ReceivedAck => {
                    format!(
                        "(State: {:?}) received ACK for packet {} from Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::ReceivedFailed => {
                    format!(
                        "(State: {:?}) failed to receive packet {} from Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::ReceivedAckFailed => {
                    format!(
                        "(State: {:?}) failed to receive ACK for packet {} from Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::ReceivedLastPacket => {
                    format!(
                        "(State: {:?}) received last packet {} from Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::Waiting => {
                    format!(
                        "(State: {:?}) waiting for packet {} from Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::Timeout => {
                    format!(
                        "(State: {:?}) timed out waiting for packet {} from Agent {}",
                        state,
                        seq_num,
                        target_agent_id.unwrap()
                    )
                }
                PacketStatus::Unknown => {
                    format!(
                        "(State: {:?}) unknown state",
                        state,
                    )
                }
            },
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
                LoggerState::Message {
                    current_agent_id, ..
                } => current_agent_id.unwrap_or(usize::MAX),
                LoggerState::Packet {
                    current_agent_id, ..
                } => current_agent_id.unwrap_or(usize::MAX),
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

            // if logger isnt message or packet and agent_id is MAX, write to log_msgs.txt
            if agent_id != usize::MAX {
                // write to the agent log file
                let path = format!("src/log/agent_{}.txt", agent_id);
                debug_file!(path, msg_buffer.as_bytes());
            } else {
                // couldnt identify the agent, so write to the general log file
                let path = format!("src/log/log_msgs.txt");
                debug_file!(path, msg_buffer.as_bytes());
            }
        }
    }
}

// TODO: Implement timeout controller
