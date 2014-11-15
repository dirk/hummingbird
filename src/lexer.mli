
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
  | Terminator

val stringify : token -> string

val lex : char Stream.t -> token Stream.t


