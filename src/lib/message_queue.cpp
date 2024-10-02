#include <iostream>
#include <queue>
#include <mutex>
#include <condition_variable>
#include <thread>
#include <chrono>
#include <optional>

// A thread-safe message queue
template <typename T>
class MessageQueue {
private:
    std::queue<T> queue_;             // Queue holding the messages
    std::mutex mtx_;                  // Mutex for thread-safe access to the queue
    std::condition_variable cond_var_; // Condition variable for blocking threads
public:
    // Push a message into the queue
    void send(T msg) {
        std::lock_guard<std::mutex> lock(mtx_);
        queue_.push(std::move(msg));
        cond_var_.notify_one();
    }

    // Try to receive a message with a timeout
    std::optional<T> recv_timeout(std::chrono::milliseconds timeout) {
        std::unique_lock<std::mutex> lock(mtx_);

        if (!cond_var_.wait_for(lock, timeout, [this] {
            return !queue_.empty(); 
        })) {
            return {};  // Timeout: return an empty optional
        }

        T msg = std::move(queue_.front());
        queue_.pop();
        return msg;
    }
};

// Function to simulate message sending after a delay
void delayed_send(MessageQueue<std::string>& mq, int delay_seconds, std::string message) {
    std::this_thread::sleep_for(std::chrono::seconds(delay_seconds));
    mq.send(std::move(message));
}

int main() {
    MessageQueue<std::string> mq;

    std::thread sender_thread(delayed_send, std::ref(mq), 2, "Hello from the thread!");

    auto result = mq.recv_timeout(std::chrono::milliseconds(1000));

    if (result) {
        std::cout << "Received: " << *result << std::endl;
    } else {
        std::cout << "Error: Timeout occurred" << std::endl;
    }

    // Wait for the sender thread to finish
    sender_thread.join();

    return 0;
}
