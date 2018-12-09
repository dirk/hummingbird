SRC = $(shell find src -type f -name '*.d')

BIN = bin/hummingbird

all: $(BIN)

clean:
	dub clean
	rm -f $(BIN)

$(BIN): $(SRC)
	dub build
