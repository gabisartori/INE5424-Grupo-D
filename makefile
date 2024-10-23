all:
	@mkdir -p tests
	@rm -f tests/*.txt
	@cargo run --release > tests/result.txt
	@clear
	@cat tests/result.txt

debug:
	@mkdir -p tests
	@rm -f tests/*.txt
	@cargo run > tests/result.txt
	@clear
	@cat tests/result.txt

clean:
	@cargo clean
