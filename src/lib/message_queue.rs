use std::sync::{Mutex, Condvar, mpsc::RecvTimeoutError};
use std::time::Duration;
use std::sync::Arc;
use std::thread;

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

    // Try to receive a message with a custom timer
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let mut queue = self.queue.lock().unwrap();

        // Start a separate timer thread to handle the timeout
        let cond_var_clone = self.cond_var.clone();
        thread::spawn(move || {
            thread::sleep(timeout);
            // Notify that the timer has expired
            cond_var_clone.notify_one(); // Notify the main thread about the timeout
        });

        // Wait for either a message or the timeout notification
        queue = self.cond_var.wait(queue).unwrap();
        
        if !queue.is_empty() {
            Ok(queue.remove(0)) // Return the message from the front of the queue
        } else {
            Err(RecvTimeoutError::Timeout) // Timeout if the queue is still empty
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
