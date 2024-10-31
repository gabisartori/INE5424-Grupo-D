BROADCAST = "AB"
TIMEOUT = 1
TIMEOUT_LIMIT = 100
MESSAGE_TIMEOUT = 50
BROADCAST_TIMEOUT = 100
GOSSIP_RATE = 3
W_SIZE = 5
LOSS_RATE = 0.01
CORRUPTION_RATE = 0.01

all:
	@mkdir -p tests
	@rm -f tests/*.txt
	@rm -rf src/log
	@rm -rf relcomm/log
	@mkdir -p src/log
	@mkdir -p relcomm/log
	@cargo build --release
	@clear
	@target/release/INE5424 $(BROADCAST) $(TIMEOUT) $(TIMEOUT_LIMIT) $(MESSAGE_TIMEOUT) $(BROADCAST_TIMEOUT) $(GOSSIP_RATE) $(W_SIZE) $(LOSS_RATE) $(CORRUPTION_RATE)
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
	@target/debug/INE5424 $(BROADCAST) $(TIMEOUT) $(TIMEOUT_LIMIT) $(MESSAGE_TIMEOUT) $(BROADCAST_TIMEOUT) $(GOSSIP_RATE) $(W_SIZE) $(LOSS_RATE) $(CORRUPTION_RATE)
	@cat tests/Resultado.txt

clean:
	@cargo clean
