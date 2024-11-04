BROADCAST = "AB"
TIMEOUT = 1
TIMEOUT_LIMIT = 10
MESSAGE_TIMEOUT = 50
BROADCAST_TIMEOUT = 100
GOSSIP_RATE = 3
W_SIZE = 5
LOSS_RATE = 0.01
CORRUPTION_RATE = 0.01

all:
	@cargo build -q --release
	@target/release/INE5424 $(BROADCAST) $(TIMEOUT) $(TIMEOUT_LIMIT) $(MESSAGE_TIMEOUT) $(BROADCAST_TIMEOUT) $(GOSSIP_RATE) $(W_SIZE) $(LOSS_RATE) $(CORRUPTION_RATE)
	@cat tests/Resultado.txt

debug:
	@cargo build -q
	@target/debug/INE5424 $(BROADCAST) $(TIMEOUT) $(TIMEOUT_LIMIT) $(MESSAGE_TIMEOUT) $(BROADCAST_TIMEOUT) $(GOSSIP_RATE) $(W_SIZE) $(LOSS_RATE) $(CORRUPTION_RATE)
	@cat tests/Resultado.txt

clean:
	@rm -rf tests/*.txt
	@rm -rf src/log
	@rm -rf relcomm/log
	@cargo clean
