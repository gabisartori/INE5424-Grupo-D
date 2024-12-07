use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use std::collections::HashMap;
// the hashmap



use relcomm::reliable_communication::ReliableCommunication;

use crate::config::{KEY_SIZE, MSG_SIZE};

/// Struct that represents the distributed hash table
pub struct DistrHash {
    communication: Arc<ReliableCommunication>,
    hash_table: Mutex<HashMap<u32, Vec<u8>>>,
}

impl DistrHash {
    pub fn new(
        communication: Arc<ReliableCommunication>,
    ) -> Arc<Self>{
        let instance= Arc::new(DistrHash {
            communication,
            hash_table: Mutex::new(HashMap::new()),
        });
        thread::spawn({
            let instance = instance.clone();
            move || {
                instance.update_table();
            }
        });
        instance
    }

    pub fn write(&self, key: &u32, msg: Vec<u8>) {
        let mut table = self.hash_table.lock().unwrap();
        let key_bytes = key.to_string();
        let key_bytes = key_bytes.as_bytes();
        let mut key_bytes = key_bytes.to_vec();
        key_bytes.extend_from_slice(msg.as_slice());        
        table.insert(key.clone(), msg);
        self.communication.broadcast(key_bytes);
        return;
    }

    pub fn read(&self, key: &u32) -> Option<Vec<u8>> {
        let table = self.hash_table.lock().unwrap();
        table.get(key).cloned()
    }

    fn update_table(&self) {
        loop {
            let mut buffer = vec![0; MSG_SIZE];
            if self.communication.receive(&mut buffer) {
                let key = u32::from_be_bytes(buffer[0..KEY_SIZE].try_into().unwrap());
                let value = buffer[KEY_SIZE..MSG_SIZE].to_vec();
                let mut table = self.hash_table.lock().unwrap();
                table.insert(key, value);
            }
        }
    }

}
