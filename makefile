all:
	@cargo run -p hashmap -q -r

test:
	@cargo run -p relcomm_tests -q -r
	@cat tests/Resultado.txt

clean:
	@rm -rf tests/
	@rm -rf src/log
	@rm -rf relcomm/log
	@cargo clean
