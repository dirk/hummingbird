
type binop =
  | Add
  | Sub
  | Div
  | Mul
  | Mod
  | Assgn

type path_item =
  | Indexer of int
  | Property of string

type expr =
  | Name of string
  | Binary of expr * binop * expr
  | Var of string * expr
  | Let of string * expr
  | Block of expr list
  | Path of string * path_item list

let offset = ref 0
let out () = offset := !offset - 2

let make_indent () =
  String.make !offset ' '

let p s =
  print_endline ((make_indent ()) ^ s)

let pin s  = p s; offset := !offset + 2
let pout s = out (); p s

open Printf

let rec print_ast ast =
  match ast with
  | Block stmts ->
      pin "{";
      ignore (List.map print_ast stmts);
      pout "}"
  | Var (name, expr) ->
      print_string (make_indent ());
      printf "var %s = " name;
      print_ast expr;
      print_endline "";
  | Let (name, expr) ->
      print_string (make_indent ());
      printf "let %s = " name;
      print_ast expr;
      print_endline "";
  | Path (name, path_items) ->
      print_string (sprintf "%s..." name)
  | Binary (lexpr, binop, rexpr) ->
      print_string (make_indent ());
      print_ast lexpr;
      print_string " ";
      print_string (match binop with
        | Assgn -> "="
        | _ -> "?"
      );
      print_string " ";
      print_ast rexpr;
      print_endline "";
  | _ ->
      p "(unknown)"
