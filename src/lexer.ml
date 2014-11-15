
let test = "Hello world!"

type keyword =
  | If
  | For
  | While

type token =
  | Word of string
  | Keyword of keyword
  | Whitespace

let new_buffer () = Buffer.create 256

let word_or_keyword word =
  match word with
  | "for" -> Keyword For
  | _     -> Word word

let rec lex = parser
  
  | [< '' '; stream >] ->
      [< 'Whitespace; lex stream >]

  | [< ' ('A' .. 'Z' | 'a' .. 'z' as c); stream >] ->
      let buffer = new_buffer () in
      Buffer.add_char buffer c;
      lex_word buffer stream

  (* Keywords *)
  (* | [< ''f'; ''o'; ''r'; stream >] -> [< 'Keyword For; lex stream >] *)

  | [< 'c >] ->
      Printf.printf "Unexpected character: '%c'\n" c;
      [< >]
  (* End of stream *)
  | [< >] -> [< >]

and lex_word buffer = parser
  | [< ' ('A' .. 'Z' | 'a' .. 'z' | '0' .. '9' | '_' as c); stream >] ->
      Buffer.add_char buffer c;
      lex_word buffer stream
  (* Any character not matched to the above will hit this rule *)
  | [< stream = lex >] ->
      let word = Buffer.contents buffer in
      let tok  = word_or_keyword word in
      [< 'tok; stream >]
