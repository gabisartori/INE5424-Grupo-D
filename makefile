all:
	@cargo run -q --release
	@cat tests/Resultado.txt

debug:
	@cargo run -q
	@cat tests/Resultado.txt

clean:
	@rm -rf tests/*.txt
	@rm -rf src/log
	@rm -rf relcomm/log
	@cargo clean
