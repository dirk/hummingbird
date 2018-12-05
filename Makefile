SRC = $(shell find src -type f -name '*.d')

all: bin/hummingbird

bin/hummingbird: $(SRC)
	dub build

src/parser/grammar.d: deps/grammar/src/grammar_generator.d
	cd deps/grammar && dub run

grammar: src/parser/grammar.d
