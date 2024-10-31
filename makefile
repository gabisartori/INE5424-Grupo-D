AGENT_NUM = 10
N_MSGS = 10
BROADCAST = "AB"
TIMEOUT = 1
TIMEOUT_LIMIT = 100
MESSAGE_TIMEOUT = 500
BROADCAST_TIMEOUT = 1000
IP = "127.0.0.1"
PORT = 3000
GOSSIP_RATE = 3
W_SIZE = 5
LOSS_RATE = 0.0
CORRUPTION_RATE = 0.0

all:
	@mkdir -p tests
	@rm -f tests/*.txt
	@rm -rf src/log
	@rm -rf relcomm/log
	@mkdir -p src/log
	@mkdir -p relcomm/log
	@cargo build --release
	@clear
	@target/release/INE5424 $(AGENT_NUM) $(N_MSGS) $(BROADCAST) $(TIMEOUT) $(TIMEOUT_LIMIT) $(MESSAGE_TIMEOUT) $(BROADCAST_TIMEOUT) $(IP) $(PORT) $(GOSSIP_RATE) $(W_SIZE) $(LOSS_RATE) $(CORRUPTION_RATE)
	@cat tests/Resultado.txt

debug:
	@mkdir -p tests
	@rm -f tests/*.txt
	@rm -rf src/log
	@rm -rf relcomm/log
	@mkdir -p src/log
	@mkdir -p relcomm/log
	@cargo build
	@clear
	@target/debug/INE5424 $(AGENT_NUM) $(N_MSGS) $(BROADCAST) $(TIMEOUT) $(TIMEOUT_LIMIT) $(MESSAGE_TIMEOUT) $(BROADCAST_TIMEOUT) $(IP) $(PORT) $(GOSSIP_RATE) $(W_SIZE) $(LOSS_RATE) $(CORRUPTION_RATE)
	@cat tests/Resultado.txt

clean:
	@cargo clean
