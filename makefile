AGENT_NUM = 10
N_MSGS = 10
BROADCAST = "URB"
TIMEOUT = 1
MESSAGE_TIMEOUT = 500
BROADCAST_TIMEOUT = 1000
IP = "127.0.0.1"
PORT = 3000
GOSSIP_RATE = 3
W_SIZE = 5
TIMEOUT_LIMIT = 100
HEARTBEAT_INTERVAL = 500
FAILURE_DETECTION_INTERVAL = 1000
all:
	@mkdir -p tests
	@rm -f tests/*.txt
	@rm -rf src/log
	@rm -rf relcomm/log
	@mkdir -p src/log
	@mkdir -p relcomm/log
	@cargo run --release -- $(AGENT_NUM) $(N_MSGS) $(BROADCAST) $(TIMEOUT) $(MESSAGE_TIMEOUT) $(BROADCAST_TIMEOUT) $(IP) $(PORT) $(GOSSIP_RATE) $(W_SIZE) $(TIMEOUT_LIMIT)
	@clear
	@cat tests/Resultado.txt

debug:
	@mkdir -p tests
	@rm -f tests/*.txt
	@rm -f src/log
	@rm -f relcomm/log
	@mkdir -p src/log
	@mkdir -p relcomm/log
	@cargo run -- $(AGENT_NUM) $(N_MSGS) $(BROADCAST) $(TIMEOUT) $(MESSAGE_TIMEOUT) $(BROADCAST_TIMEOUT) $(IP) $(PORT) $(GOSSIP_RATE) $(W_SIZE) $(TIMEOUT_LIMIT)
	@clear
	@cat tests/Resultado.txt

clean:
	@cargo clean
