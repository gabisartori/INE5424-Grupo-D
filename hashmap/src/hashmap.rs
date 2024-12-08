use std::sync::{Arc, Mutex};
use std::thread;
use std::io::Error;


use std::collections::HashMap;
use relcomm::reliable_communication::ReliableCommunication;

use crate::formatter;

/// Struct that represents the distributed hash table
pub struct DistrHash {
    communication: Arc<ReliableCommunication>,
    hash_table: Mutex<HashMap<String, String>>,
}

impl DistrHash {
    pub fn new(
        communication: Arc<ReliableCommunication>,
    ) -> (Arc<Self>, thread::JoinHandle<()>) {
        let instance= Arc::new(DistrHash {
            communication,
            hash_table: Mutex::new(HashMap::new()),
        });
        let listener_handle = thread::spawn({
            let instance = instance.clone();
            move || {
                instance.listener();
            }
        });
        (instance, listener_handle)
    }

    pub fn write(&self, key: &String, msg: &String) -> Result<(), Error> {
        let bytes = formatter::to_bytes(key, msg)?;
        self.communication.broadcast(bytes);
        Ok(())
    }

    pub fn read(&self, key: &String) -> Option<String> {
        let table = self.hash_table.lock().unwrap();
        table.get(key).cloned()
    }

    fn listener(&self) {
        loop {
            let mut buffer = vec![];
            if !self.communication.receive(&mut buffer) { break ; }
            match formatter::from_bytes(buffer) {
                Ok((key, value)) => {
                    let mut table = self.hash_table.lock().unwrap();
                    table.insert(key, value);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }

}
