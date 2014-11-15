
type keyword =
  | If
  | For
  | While

type token =
  | Word of string
  | Keyword of keyword
  | Whitespace

let new_buffer () = Buffer.create 256

let stringify token =
  match token with
  | Keyword If    -> "if"
  | Keyword For   -> "for"
  | Keyword While -> "while"
  | Word _        -> "word"
  | _             -> ""

let word_or_keyword word =
  match word with
  | "while" -> Keyword While
  | "for"   -> Keyword For
  | "if"    -> Keyword If
  | _       -> Word word


let rec lex = parser
  (* Gobble up white space *)
  | [< '' '; stream >] ->
      [< 'Whitespace; lex stream >]

  (* Matching words and keywords *)
  | [< ' ('A' .. 'Z' | 'a' .. 'z' as c); stream >] ->
      let buffer = new_buffer () in
      Buffer.add_char buffer c;
      lex_word buffer stream

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
      (* Figure out whether it's a word or a keyword *)
      let tok  = word_or_keyword word in
      [< 'tok; stream >]
