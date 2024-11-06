all:
	@RUST_BACKTRACE=1 cargo run -q --release
	@cat tests/Resultado.txt

debug:
	@RUST_BACKTRACE=1 cargo run -q
	@cat tests/Resultado.txt

clean:
	@rm -rf tests/*.txt
	@rm -rf src/log
	@rm -rf relcomm/log
	@cargo clean
