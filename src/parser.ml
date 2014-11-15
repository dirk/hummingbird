
open Lexer

let bad_token token =
  let str = Printf.sprintf "Unexpected token: %s" (Lexer.stringify token) in
  raise (Stream.Error str)

let parse_expression = parser
  | [< 'Lexer.Word name >] ->
      Ast.Name name
  | [< 'token >] -> bad_token token


(* Recursively consumes whitespace tokens *)
let rec skip_space stream =
  let t = Stream.peek stream in
  begin match t with
    | Some Lexer.Whitespace ->
        ignore (Stream.next stream);
        ignore (skip_space stream)
    | _ -> ()
  end


(* Take out the whitespace and equals sign in an assignment *)
let parse_assignment_equals stream =
  skip_space stream;
  let token = Stream.next stream in
  begin
    match token with
    | Lexer.Equals -> ()
    | _ -> bad_token token
  end;
  skip_space stream;
  [< >]


(* Parses the equality sign and right-hand-side expression of an assignment
 * operation.
 * TODO: Implement parsing of the type signature. 
 *)
let parse_assignment kind = parser
  | [< 'Lexer.Word name; _ = parse_assignment_equals; expr = parse_expression >] ->
      begin match kind with
        | Keyword Var -> Ast.Var (name, expr)
        | Keyword Let -> Ast.Let (name, expr)
        | _ -> assert false
      end
  | [< 'token >] -> bad_token token


let parse_statement = parser
  (* `var` definition *)
  | [< 'Keyword Var as kind; stream >] ->
      (* Need to get the variable name and then the equals *)
      parse_assignment kind stream
  (* `let` definition *)
  | [< 'Keyword Let as kind; stream >] -> parse_assignment kind stream
  | [< 'token >] -> bad_token token

(* Every program is structured as a sequence of statements, so we'll start
 * by trying to parse a statement. *)
let rec parse = parser
  | [< stmt = parse_statement; stream >] ->
      let head  = stmt in
      let tail  = [] in (* TODO: Make this parse another statement! *)
      let stmts = head :: tail in
      Ast.Block stmts

