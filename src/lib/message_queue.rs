use std::sync::{Mutex, Condvar, mpsc::RecvTimeoutError};
use std::time::Duration;
use std::sync::Arc; 
// use std::thread;

// Define a thread-safe message queue using Mutex and Condvar
#[derive(Clone)]
pub struct MessageQueue<T> {
    // Mutex-protected queue
    queue: Arc<Mutex<Vec<T>>>,
    // Condition variable to signal when a message is available
    cond_var: Arc<Condvar>,
}

impl<T> MessageQueue<T> {
    pub fn new() -> Self {
        MessageQueue {
            // Initialize an empty queue
            queue: Arc::new(Mutex::new(Vec::new())),
            // Initialize the condition variable
            cond_var: Arc::new(Condvar::new()),
        }
    }

    // Send a message into the queue and notify waiting threads
    pub fn send(&self, msg: T) -> Result<(), RecvTimeoutError> {
        {
            let mut queue = self.queue.lock().unwrap();
            queue.push(msg);
        }
        // Notify one waiting thread that a message is available
        self.cond_var.notify_one();
        Ok(())
    }

    // Try to receive a message with a timeout
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let mut queue = self.queue.lock().unwrap();
        
        // If the queue is empty, we will wait for the specified timeout
        let result: (std::sync::MutexGuard<'_, Vec<T>>, std::sync::WaitTimeoutResult) = self.cond_var.wait_timeout(queue, timeout).unwrap();
        queue = result.0;  // Re-acquire the lock after waiting

        // Check if we received a message or timed out
        if !queue.is_empty() {
            Ok(queue.remove(0)) // Return the message from the front of the queue
        } else {// Timeout if the queue is still empty
            Err(RecvTimeoutError::Timeout)
        }
    }
    pub fn recv(&self) -> Result<T, RecvTimeoutError> {
        let mut queue = self.queue.lock().unwrap();
        let result: std::sync::MutexGuard<'_, Vec<T>> = self.cond_var.wait_while(queue, |queue| queue.is_empty()).unwrap();
        queue = result;
        if !queue.is_empty() {
            Ok(queue.remove(0))
        } else {
            Err(RecvTimeoutError::Timeout)
        }
    }
    // Try to receive a message without waiting
    pub fn try_recv(&self) -> Result<T, RecvTimeoutError> {
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_empty() {
            Ok(queue.remove(0))
        } else {
            Err(RecvTimeoutError::Timeout)
        }
    }
}

// fn delayed_send(mq: Arc<MessageQueue<String>>, delay_seconds: u64, message: String) {
//     thread::sleep(Duration::from_secs(delay_seconds)); // Simulate delay
//     mq.send(message);                                  // Send the message
// }

// fn main() {
//     let mq = Arc::new(MessageQueue::new());

//     // Spawn a thread that will send a message after 2 seconds
//     let mq_clone = mq.clone();
//     thread::spawn(move || {
//         delayed_send(mq_clone, 1, "Hello from the thread!".to_string());
//     });

//     // Try to receive a message with a timeout of 1 second
//     match mq.recv_timeout(Duration::from_secs(2)) {
//         Ok(msg) => println!("Received: {}", msg),
//         Err(err) => println!("Error: {}", err),
//     }
// }
