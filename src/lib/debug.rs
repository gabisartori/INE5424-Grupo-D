// Implements Debugger for creating a debug log file

use std::fs::File;

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
// }

#[derive(Debug, Clone, Copy)]
pub enum PacketStatus {
    Sent,
    Received,
    SentFailed,
    ReceivedFailed,
    Waiting,
    LastPacket,
    Fragmenting,
}

pub enum LoggerState {
    PacketSender {
        state: PacketStatus,
        current_agent_id: u16,
        target_agent_id: u16,
        seq_num: u32,
        action: PacketStatus,
    },

    // AgentState {
    //     state: AgentStatus,
    //     agent_id: u16,
    // },

    MessageReceiver {
        state: PacketStatus,
        current_agent_id: u16,
        target_agent_id: u16,
        message_id: u32,
        action: PacketStatus,
    },
    MessageSender {
        state: PacketStatus,
        current_agent_id: u16,
        target_agent_id: u16,
        message_id: u32,
        action: PacketStatus,
    },
}

// Define the structure for the debug log
#[derive(Debug)]
pub struct DebugLog {}

impl DebugLog {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_log(&self, log_type: LoggerState) -> String {
        match log_type {
            LoggerState::PacketSender {
                state,
                current_agent_id,
                target_agent_id,
                seq_num,
                action,
            } => {
                format!(
                    "Agent {} , state: {:?}, sending packet {} to Agent {}. Next action : {:?}",
                    current_agent_id, state, seq_num, target_agent_id, action
                )
            }
            // LoggerState::AgentState { state, agent_id } => {
            //     format!("Agent {} , state: {:?}", agent_id, state)
            // }
            LoggerState::MessageReceiver {
                state,
                current_agent_id,
                target_agent_id,
                message_id,
                action,
            } => {
                format!(
                    "Agent {} , state: {:?}, receiving message {} from Agent {}. Next action : {:?}",
                    current_agent_id, state, message_id, target_agent_id, action
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
                    "Agent {} , state: {:?}, sending message {} to Agent {}. Next action : {:?}",
                    current_agent_id, state, message_id, target_agent_id, action
                )
            }
        }
    }
}

pub struct Debugger {
    debug_level: u8,
    log_file: File,
}

impl Debugger {
    pub fn new(debug_level: u8) -> Self {
        Self {
            debug_level,
            log_file: File::create("tests/debug.txt").unwrap(),
        }
    }

    pub fn log(&mut self, log: DebugLog, logger_state: LoggerState) {
        let log = log.get_log(logger_state);
        let msg_buffer = format!("{}\n", log);
        if self.debug_level > 0 {
            std::io::Write::write_all(&mut self.log_file, msg_buffer.as_bytes())
                .expect("Erro ao escrever no arquivo");
        }
    }
}

// TODO : implement a simulator for agent actions
