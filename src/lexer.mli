
type keyword =
  | If
  | For
  | While

type token =
  | Word of string
  | Keyword of keyword
  | Whitespace

val stringify : token -> string

val lex : char Stream.t -> token Stream.t


