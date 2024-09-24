all:
	@clear
	@rm -f tests/*.txt
	@cargo run --release > tests/log.txt
	@echo "------------------------------------------------------------"
	@cat tests/log.txt

debug:
	@clear
	@rm -f tests/*.txt
	@cargo run > tests/log.txt

clean:
	@cargo clean

test:
	for i in {1..100};	do cargo run > tests/log.txt; done
