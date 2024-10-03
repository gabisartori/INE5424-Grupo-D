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

# compiles src/lib/message_queue.cpp in cpp_out
# runs cpp_out with 2 args received from user
# removes cpp_out
arg1 = $(word 2, $(MAKECMDGOALS))
arg2 = $(word 3, $(MAKECMDGOALS))
cpp:
	@rm -f tests/*.txt
	@g++ -o cpp_out src/lib/message_queue.cpp
	@./cpp_out $(arg1) $(arg2)
	@rm -f cpp_out

test:
	for i in {1..100};	do cargo run > tests/log.txt; done
