
type keyword =
  | If
  | For
  | While
  | Let
  | Var

type token =
  | Word of string
  | Keyword of keyword
  | Whitespace
  | LParen
  | RParen
  | LSquare
  | RSquare
  | LBracket
  | RBracket
  | Equals

let new_buffer () = Buffer.create 256

let stringify token =
  match token with
  | Keyword If    -> "if"
  | Keyword For   -> "for"
  | Keyword While -> "while"
  | Keyword Let   -> "let"
  | Keyword Var   -> "var"
  | Word _        -> "word"
  | LParen        -> "("
  | RParen        -> ")"
  | LSquare       -> "["
  | RSquare       -> "]"
  | LBracket      -> "{"
  | RBracket      -> "}"
  | Whitespace    -> "whitespace"
  | Equals        -> "="
  (*| _             -> "(unknown)"*)

let word_or_keyword word =
  match word with
  | "while" -> Keyword While
  | "for"   -> Keyword For
  | "if"    -> Keyword If
  | "var"   -> Keyword Var
  | _       -> Word word

let rec skip_space stream =
  let c = Stream.peek stream in
  match c with
  | Some c ->
      if c == ' ' || c == '\t' then begin
        ignore (Stream.next stream);
        skip_space stream
      end;
  | None -> ()

let rec lex = parser
  (* Gobble up white space *)
  | [< ' (' ' | '\t'); stream >] -> chomp_space stream

  (* Matching words and keywords *)
  | [< ' ('A' .. 'Z' | 'a' .. 'z' as c); stream >] ->
      let buffer = new_buffer () in
      Buffer.add_char buffer c;
      lex_word buffer stream

  (* Get control tokens *)
  | [< ''('; stream >] -> [< 'LParen;   lex stream >]
  | [< '')'; stream >] -> [< 'RParen;   lex stream >]
  | [< ''{'; stream >] -> [< 'LBracket; lex stream >]
  | [< ''}'; stream >] -> [< 'RBracket; lex stream >]
  | [< ''['; stream >] -> [< 'LSquare;  lex stream >]
  | [< '']'; stream >] -> [< 'RSquare;  lex stream >]
  | [< ''='; stream >] -> [< 'Equals;   lex stream >]

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
  | [< stream >] ->
      let word = Buffer.contents buffer in
      (* Figure out whether it's a word or a keyword *)
      let tok  = word_or_keyword word in
      (* Gobble space if it's a keyword *)
      begin
        match tok with
        | Keyword _ -> skip_space stream; ()
        | _ -> ()
      end;
      (* Add the new word/keyword to the stream *)
      [< 'tok; lex stream >]

and chomp_space = parser
  (* Eat up space *)
  | [< ' (' ' | '\t'); stream >] ->
      chomp_space stream
  (* Add a whitespace token once we reach the end of this run of whitespace *)
  | [< stream = lex >] ->
      [< 'Whitespace; stream >]
