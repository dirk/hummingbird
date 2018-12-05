SRC = $(shell find src -type f -name '*.d')

BIN = bin/hummingbird

all: $(BIN)

clean:
	dub clean
	rm -f $(BIN)

grammar: src/parser/grammar.d

$(BIN): $(SRC)
	dub build

src/parser/grammar.d: deps/grammar/src/grammar_generator.d
	cd deps/grammar && dub run
