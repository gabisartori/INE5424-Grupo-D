use std::sync::{Arc, Mutex, Condvar, mpsc::RecvTimeoutError};
use std::thread;
use std::time::Duration;

// deriva clone
// Define a thread-safe message queue using Mutex and Condvar
pub struct MessageQueue<T> {
    queue: Mutex<Vec<T>>,         // Mutex-protected queue
    cond_var: Condvar,            // Condition variable to signal when a message is available
}

impl<T> MessageQueue<T> {
    pub fn new() -> Self {
        MessageQueue {
            queue: Mutex::new(Vec::new()),    // Initialize an empty queue
            cond_var: Condvar::new(),         // Initialize the condition variable
        }
    }

    // Send a message into the queue and notify waiting threads
    pub fn send(&self, msg: T) -> Result<(), RecvTimeoutError> {
        let mut queue = self.queue.lock().unwrap();
        queue.push(msg);
        self.cond_var.notify_one();           // Notify one waiting thread that a message is available
        Ok(())
    }

    // Try to receive a message with a timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let mut queue = self.queue.lock().unwrap();
        
        // If the queue is empty, we will wait for the specified timeout
        let result = self.cond_var.wait_timeout(queue, timeout).unwrap();
        queue = result.0;  // Re-acquire the lock after waiting

        // Check if we received a message or timed out
        if !queue.is_empty() {
            Ok(queue.remove(0)) // Return the message from the front of the queue
        } else {// Timeout if the queue is still empty
            Err(RecvTimeoutError::Timeout)
        }
    }
}

fn delayed_send(mq: Arc<MessageQueue<String>>, delay_seconds: u64, message: String) {
    thread::sleep(Duration::from_secs(delay_seconds)); // Simulate delay
    mq.send(message);                                  // Send the message
}

fn main() {
    let mq = Arc::new(MessageQueue::new());

    // Spawn a thread that will send a message after 2 seconds
    let mq_clone = mq.clone();
    thread::spawn(move || {
        delayed_send(mq_clone, 1, "Hello from the thread!".to_string());
    });

    // Try to receive a message with a timeout of 1 second
    match mq.recv_timeout(Duration::from_secs(2)) {
        Ok(msg) => println!("Received: {}", msg),
        Err(err) => println!("Error: {}", err),
    }
}
