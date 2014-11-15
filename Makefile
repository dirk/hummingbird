
main.native: main.ml src/*.ml
	ocamlbuild -use-ocamlfind -I src $@

default:: main.native

clean::
	rm main.native

cleanall:
	rm -r _build
