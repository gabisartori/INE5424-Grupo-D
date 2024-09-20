all:
	@clear
	@rm -f target/*.txt
	@cargo run > log.txt

clean:
	@cargo clean
