
type keyword =
  | If
  | For
  | While

type token =
  | Word of string
  | Keyword of keyword
  | Whitespace

val test : string

val lex : char Stream.t -> token Stream.t
