// Implements Debugger for creating a debug log file

use std::fs::File;

// Define states for the agents
// #[derive(Debug, Clone, Copy)]
// enum AgentState {
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

enum LoggerState {

    packet_sender {
        state: AgentState,
        current_agent_id: u16,
        target_agent_id: u16,
        seq_num: u32,
        action: AgentState,
    },

    agent_state {
        state: AgentState,
        agent_id: u16,
    },

    message_receiver {
        state: AgentState,
        current_agent_id: u16,
        target_agent_id: u16,
        message_id: u32,
        action: AgentState,
    },
    message_sender {
        state: AgentState,
        current_agent_id: u16,
        target_agent_id: u16,
        message_id: u32,
        action: AgentState,
    },
}

// Define the structure for the debug log
#[derive(Debug)]
pub struct DebugLog {
}

impl DebugLog {
    pub fn new(log_type: LoggerState) -> Self {

    }

    pub fn get_log(&self, log_type: LoggerState) -> String {
        match log_type {
            LoggerState::packet_sender {
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
            LoggerState::agent_state { state, agent_id } => {
                format!("Agent {} , state: {:?}", agent_id, state)
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
            log_file: File::create("debug.txt").unwrap(),
        }
    }

    pub fn log(&self, log: DebugLog) {
        if self.debug_level > 0 {
            println!("{}", log.get_log());
        }
    }
}

// TODO : implement a simulator for agent actions
