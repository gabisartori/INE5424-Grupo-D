#include <atomic>
#include <mutex>
#include <vector>
#include <chrono>
#include <sys/syscall.h>
#include <linux/futex.h>
#include <unistd.h>
#include <time.h>
#include <iostream>
#include <thread>
#include <cerrno>
#include <optional>

template <typename T>
class MessageQueue {
private:
    std::vector<T> queue;          // Queue for messages
    std::mutex queue_mutex;        // Mutex for queue protection
    std::atomic<int32_t> futex_var; // Futex variable for synchronization

    // Futex helper functions
    static int futex_wait(std::atomic<int32_t>* addr, int32_t val, const struct timespec* timeout) {
        return syscall(SYS_futex, addr, FUTEX_WAIT, val, timeout, nullptr, 0);
    }

    static int futex_wake(std::atomic<int32_t>* addr, int32_t count) {
        return syscall(SYS_futex, addr, FUTEX_WAKE, count);
    }

public:
    MessageQueue() : futex_var(0) {}  // Initialize futex to 0 (wait state)

    // Send a message to the queue
    void send(const T& msg) {
        {
            std::lock_guard<std::mutex> lock(queue_mutex);
            queue.push_back(msg);  // Push the message into the queue
        }

        futex_var.store(1, std::memory_order_seq_cst);  // Set futex to signal state
        futex_wake(&futex_var, 1);  // Wake up one waiting thread
    }

    // Try to receive a message with a timeout, returns an optional
    std::optional<T> recv_timeout(std::chrono::milliseconds timeout) {
        {
            std::lock_guard<std::mutex> lock(queue_mutex);
            if (!queue.empty()) {
                T msg = queue.front();
                queue.erase(queue.begin());  // Remove the message from the queue
                futex_var.store(0, std::memory_order_seq_cst);  // Reset futex after message is consumed
                return msg;
            }
        }

        // Set up timespec for timeout
        struct timespec ts;
        ts.tv_sec = std::chrono::duration_cast<std::chrono::seconds>(timeout).count();
        ts.tv_nsec = std::chrono::duration_cast<std::chrono::nanoseconds>(timeout).count() % 1000000000;

        int32_t futex_val = futex_var.load(std::memory_order_seq_cst);
        if (futex_wait(&futex_var, futex_val, &ts) == -1) {
            if (errno == ETIMEDOUT) {
                return std::nullopt;  // Timeout, return null optional
            }
        }

        // Re-acquire the lock and check the queue again
        std::lock_guard<std::mutex> lock(queue_mutex);
        if (!queue.empty()) {
            T msg = queue.front();
            queue.erase(queue.begin());  // Remove the message from the queue
            futex_var.store(0, std::memory_order_seq_cst);  // Reset futex after message is consumed
            return msg;
        }

        return std::nullopt;  // Timeout, return null optional
    }

    // Receive a message, blocking indefinitely until one is available
    std::optional<T> recv() {
        {
            std::lock_guard<std::mutex> lock(queue_mutex);
            if (!queue.empty()) {
                T msg = queue.front();
                queue.erase(queue.begin());  // Remove the message from the queue
                futex_var.store(0, std::memory_order_seq_cst);  // Reset futex after message is consumed
                return msg;
            }
        }

        int32_t futex_val = futex_var.load(std::memory_order_seq_cst);
        futex_wait(&futex_var, futex_val, nullptr);  // Wait indefinitely on futex

        // Re-acquire the lock and check the queue
        std::lock_guard<std::mutex> lock(queue_mutex);
        if (!queue.empty()) {
            T msg = queue.front();
            queue.erase(queue.begin());  // Remove the message from the queue
            futex_var.store(0, std::memory_order_seq_cst);  // Reset futex after message is consumed
            return msg;
        }

        return std::nullopt;  // No message available
    }

    // Non-blocking receive attempt
    std::optional<T> try_recv() {
        std::lock_guard<std::mutex> lock(queue_mutex);
        if (!queue.empty()) {
            T msg = queue.front();
            queue.erase(queue.begin());  // Remove the message from the queue
            futex_var.store(0, std::memory_order_seq_cst);  // Reset futex after message is consumed
            return msg;
        }
        return std::nullopt;  // No message available
    }
};

// receive the timeouts from the args
int main (int argc, char *argv[]) {
    if (argc != 3) {
        std::cerr << "Usage: " << argv[0] << " <timeout_send_in_seconds> <timeout_recv_in_seconds>" << std::endl;
        return 1;
    }

    int t1 = std::stoi(argv[1]);
    int t2 = std::stoi(argv[2]);
    
    MessageQueue<std::string> mq;

    std::thread sender([&mq, t1]() {
        std::this_thread::sleep_for(std::chrono::seconds(t1));
        mq.send("Hello from C++ thread!");
    });

    auto result = mq.recv_timeout(std::chrono::seconds(t2));
    if (result) {
        std::cout << "Received: " << *result << std::endl;
    } else {
        std::cout << "Timeout occurred." << std::endl;
    }

    sender.join();
    return 0;
}
