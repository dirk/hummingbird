
open Lexer

let bad_token token =
  let str = Printf.sprintf "Unexpected token: %s" (Lexer.stringify token) in
  raise (Stream.Error str)

let bad_token_scope scope token =
  let str = Printf.sprintf "%s: unexpected token: %s" scope (Lexer.stringify token) in
  raise (Stream.Error str)  

let bad_eof () =
  raise (Stream.Error "Unexpected end of input")


(* Recursively consumes whitespace tokens *)
let rec skip_space stream =
  let t = Stream.peek stream in
  begin match t with
    | Some Lexer.Whitespace ->
        ignore (Stream.next stream);
        ignore (skip_space stream)
    | _ -> ()
  end

let parse_path name stream =
  let path = Ast.Path (name, []) in
  path


let lexer_op_to_ast op =
  match op with
  | Lexer.Equals -> Ast.Assgn
  | _ -> assert false

let is_lexer_binop op =
  match op with
  | Lexer.Equals -> true
  | _ -> false



let rec check_binary current stream =
  let binary = try_parse_binary_expression current stream in
  match binary with
  (* If the binary parser found something then return that *)
  | Some expr ->
      expr
  (* Otherwise return just what we have *)
  | None ->
      current

(* Returns an optional Binary if it was able to parse a binary expression *)
and try_parse_binary_expression lhs stream =
  let next = Stream.peek stream in
  match next with
  | Some Whitespace ->
      (* If the next is a binary operator, and if so move past the whitespace
       * and parse the binop. *)
      let next2 = Stream.npeek 2 stream in
      let op    = List.nth next2 1 in

      if is_lexer_binop op
      then begin
        (* Move past whitespace *)
        ignore (Stream.next stream);
        Some (parse_bexp_rhs lhs stream)
      end
      else None
  | _ -> None

(* Parse the operator and right-hand-side of a binary expression. *)
and parse_bexp_rhs lhs stream =
  let op  = lexer_op_to_ast (Stream.next stream) in
  (* Then clear out any whitespace after the op token *)
  skip_space stream;
  (* Then parse the right *)
  let rhs = parse_expression stream in
  Ast.Binary (lhs, op, rhs)

(* Simple expressions *)
and parse_expression = parser
  | [< 'Lexer.Word name; stream >] ->
      let path = parse_path name stream in
      check_binary path stream
  | [< 'token >] -> bad_token_scope "parse_expression" token



(* Take out the whitespace and equals sign in an assignment *)
let parse_assignment_equals stream =
  skip_space stream;
  let token = Stream.next stream in
  begin match token with
    | Lexer.Equals -> ()
    | _ -> bad_token token
  end;
  skip_space stream;
  [< >]


(* Parses the name, equality sign and right-hand-side expression of a
 * var/let assignment operation.
 * TODO: Implement parsing of the type signature. 
 *)
let parse_assignment kind = parser
  | [< 'Lexer.Word name; _ = parse_assignment_equals; expr = parse_expression >] ->
      begin match kind with
        | Keyword Var -> Ast.Var (name, expr)
        | Keyword Let -> Ast.Let (name, expr)
        | _ -> assert false
      end
  | [< 'token >] -> bad_token_scope "parse_assignment" token

let parse_statement = parser
  (* `var` definition *)
  | [< 'Keyword Var as kind; stream >] ->
      (* Need to get the variable name and then the equals *)
      let assgn = parse_assignment kind stream in
      skip_space stream; assgn

  (* `let` definition *)
  | [< 'Keyword Let as kind; stream >] ->
      let assgn = parse_assignment kind stream in
      skip_space stream; assgn

  (* If the statement starts with a name then we need to figure out how to
   * parse the rest of it:
   *   - Assignment
   *   - Function call *)
  | [< expr = parse_expression >] ->
      expr

  | [< 'token >] -> bad_token_scope "parse_statement" token

let rec parse_statements = parser
  | [< stmt = parse_statement; stream >] ->
      let head = stmt in
        begin
          let token = Stream.peek stream in
          match token with
          | Some Terminator ->
              (* Gobble the terminator *)
              ignore (Stream.next stream); ()
          | _ -> ()
        end;
      let tail  = parse_statements stream in (* TODO: Make this parse another statement! *)
      let stmts = head :: tail in
      stmts
  | [< >] -> []

(* Every program is structured as a sequence of statements, so we'll start
 * by trying to parse a statement. *)
let parse stream =
  let stmts = parse_statements stream in
  Ast.Block stmts
