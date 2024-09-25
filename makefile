all:
	@rm -f tests/*.txt
	@cargo run --release > tests/log.txt
	@clear
	@echo "------------------------------------------------------------"
	@cat tests/log.txt
	@echo "------------------------------------------------------------"
	@python3 src/calculate_test.py

debug:
	@rm -f tests/*.txt
	@cargo run > tests/log.txt
	@clear
	@echo "------------------------------------------------------------"
	@python3 src/calculate_test.py

clean:
	@cargo clean

test:
	for i in {1..100};	do cargo run > tests/log.txt; done
