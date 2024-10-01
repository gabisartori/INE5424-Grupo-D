all:
	@rm -f tests/*.txt
	@cargo run --release > result.txt
	@clear
	@cat result.txt
	@rm -f result.txt

debug:
	@rm -f tests/*.txt
	@cargo run > result.txt
	@clear
	@cat result.txt
	@rm -f result.txt

clean:
	@cargo clean
