use std::sync::{Mutex, mpsc::RecvTimeoutError};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use libc::{timespec, syscall, SYS_futex, FUTEX_WAIT, FUTEX_WAKE};
use std::ptr;

#[derive(Clone)]
pub struct MessageQueue<T> {
    queue: Arc<Mutex<Vec<T>>>,
    futex_var: Arc<AtomicI32>,  // Use futex for synchronization
}

impl<T> MessageQueue<T> {
    pub fn new() -> Self {
        MessageQueue {
            queue: Arc::new(Mutex::new(Vec::new())),
            futex_var: Arc::new(AtomicI32::new(0)),  // Initialize futex to 0 (wait state)
        }
    }

    pub fn send(&self, msg: T) -> Result<(), RecvTimeoutError> {
        {
            let mut queue = self.queue.lock().unwrap();
            queue.push(msg);  // Push the message into the queue
        }
        
        self.futex_var.store(1, Ordering::SeqCst);  // Set futex value to 1 (signal state)
        unsafe {
            syscall(SYS_futex, self.futex_var.as_ptr(), FUTEX_WAKE, 1, ptr::null::<libc::timespec>());
        }
        Ok(())
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        {
            let mut queue = self.queue.lock().unwrap();
            // If there's already a message, return it immediately
            if !queue.is_empty() {
                return Ok(queue.remove(0));
            }
        }

        // Set up timespec structure for timeout
        let timeout_sec = timeout.as_secs() as i64;
        let timeout_nsec = timeout.subsec_nanos() as i64;
        let ts = timespec {
            tv_sec: timeout_sec,
            tv_nsec: timeout_nsec,
        };

        let futex_val = self.futex_var.load(Ordering::SeqCst);

        // Wait using futex
        unsafe {
            let res = syscall(
                SYS_futex,
                self.futex_var.as_ptr(),
                FUTEX_WAIT,
                futex_val,
                &ts as *const _
            );

            if res == -1 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::ETIMEDOUT) {
                    return Err(RecvTimeoutError::Timeout);
                }
                // Handle other errors if needed
            }
        }

        // After waking up, check the queue again
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_empty() {
            self.futex_var.store(0, Ordering::SeqCst);  // Reset futex after consuming the message
            return Ok(queue.remove(0));
        }

        // If queue is still empty, return timeout error
        Err(RecvTimeoutError::Timeout)
    }

    pub fn recv(&self) -> Result<T, RecvTimeoutError> {
        {
            let mut queue = self.queue.lock().unwrap();    
            // If the queue is not empty, return the first message
            if !queue.is_empty() {
                self.futex_var.store(0, Ordering::SeqCst);  // Reset futex after consuming the message
                return Ok(queue.remove(0));
            }
        }

        let futex_val = self.futex_var.load(Ordering::SeqCst);

        // Wait on the futex until a message is available
        unsafe {
            syscall(SYS_futex, self.futex_var.as_ptr(), FUTEX_WAIT, futex_val, ptr::null::<libc::timespec>());
        }

        // Re-acquire the lock and return the message once available
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_empty() {
            self.futex_var.store(0, Ordering::SeqCst);  // Reset futex after consuming the message
            Ok(queue.remove(0))
        } else {
            Err(RecvTimeoutError::Timeout)
        }
    }

    pub fn try_recv(&self) -> Result<T, RecvTimeoutError> {
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_empty() {
            self.futex_var.store(0, Ordering::SeqCst);  // Reset futex after consuming the message
            Ok(queue.remove(0))
        } else {
            Err(RecvTimeoutError::Timeout)
        }
    }
}
