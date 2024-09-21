all:
	@cargo run --release > tests/log.txt

debug:
	@clear
	@rm -f tests/*.txt
	@cargo run > tests/log.txt

clean:
	@cargo clean
