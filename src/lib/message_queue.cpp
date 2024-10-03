#include <iostream>
#include <queue>
#include <thread>
#include <mutex>
#include <condition_variable>
#include <chrono>
#include <optional>

class my_timespec {
public:
    time_t tv_sec;
    long tv_nsec;
};

void my_nanosleep(const my_timespec* req, my_timespec*) {
    std::this_thread::sleep_for(std::chrono::seconds(req->tv_sec) + std::chrono::nanoseconds(req->tv_nsec));
}

template <typename T>
class MessageQueue {
private:
    std::queue<T> queue;
    std::mutex queue_mutex;
    std::condition_variable cond_var;

public:
    // Add a message to the queue and notify waiting threads
    void send(const T& msg) {
        {
            std::lock_guard<std::mutex> lock(queue_mutex);
            queue.push(msg);
        }
        cond_var.notify_one();
    }

    // Receive a message with a timeout
    std::optional<T> recv_timeout(std::chrono::seconds timeout) {
        std::unique_lock<std::mutex> lock(queue_mutex);

        // Timer thread to handle timeout
        std::thread timer_thread([&]() {
            my_timespec ts;
            ts.tv_sec = timeout.count();
            ts.tv_nsec = 0;
            my_nanosleep(&ts, nullptr);
            cond_var.notify_one();
        });

        // Wait for either a message or the timeout
        if (cond_var.wait_for(lock, timeout, [&] { return !queue.empty(); })) {
            T msg = queue.front();
            queue.pop();
            timer_thread.detach();
            return msg; // Return the message
        } else {
            timer_thread.detach();
            return std::nullopt; // Timeout occurred, no message
        }
    }

    // Receive a message without timeout (blocking until a message is available)
    T recv() {
        std::unique_lock<std::mutex> lock(queue_mutex);
        cond_var.wait(lock, [&] { return !queue.empty(); });
        T msg = queue.front();
        queue.pop();
        return msg; // Return the message
    }

    // Try to receive a message without blocking
    std::optional<T> try_recv() {
        std::lock_guard<std::mutex> lock(queue_mutex);
        if (!queue.empty()) {
            T msg = queue.front();
            queue.pop();
            return msg; // Return the message
        } else {
            return std::nullopt; // No message available
        }
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
