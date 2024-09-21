all:
	@clear
	@rm -f tests/*.txt
	@cargo run > tests/log.txt

clean:
	@cargo clean
