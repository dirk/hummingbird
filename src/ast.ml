
type binop =
  | Add
  | Sub
  | Div
  | Mul
  | Mod

type expr =
  | Name of string
  | Binary of expr * binop * expr
  | Var of string * expr
  | Let of string * expr
  | Block of expr list

let offset = ref 0
let p s =
  let indent = String.make !offset ' ' in
  print_endline (indent ^ s)

let pin s  = p s; offset := !offset + 2
let pout s = offset := !offset - 2; p s

let rec print_ast ast =
  match ast with
  | Block stmts ->
      pin "{";
      ignore (List.map print_ast stmts);
      pout "}"
  | Var (name, expr) ->
      pin (Printf.sprintf "var %s = (" name);
      print_ast expr;
      pout ")"
  | Name name ->
      p name
  | _ ->
      p "(unknown)"