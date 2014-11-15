
(* External dependencies *)
open Core;;

(* Internal modules *)
(* open Lexer;; *)

let readfile filename = In_channel.read_all filename

let parse contents = contents

let debug_dump_parse tree =
  print_endline "Hello world!"

let usage () =
  print_endline "Usage: hb file"

let end_of_stream stream =
  (Stream.peek stream) == None

let main () =
  (* "var a = b\nc = d\nif e { }\n" *)
  (* "var a = b\nlet c = d\ne = f\n" *)
  let lex_stream = Lexer.lex (Stream.of_string "a = b\nvar c = d\n") in
  try begin
    let ast = Parser.parse lex_stream in
    Ast.print_ast ast;
    ()
    (* while true do
      let token = Stream.next lex_stream in
      print_string (Lexer.stringify token);
      print_string " "
    done *)
  end with
  | Stream.Failure ->
    if not (end_of_stream lex_stream)
    then raise (Stream.Error "Failed reading stream")
    else print_endline "";
  | Stream.Error err ->
    Printf.printf "Parser error at index: %d (%s)\n" (Stream.count lex_stream) err;

  (*
  let rec loop stream =
    try begin
      let token = Stream.next stream in
      begin
        match token with
        | Lexer.Word t ->
            print_string ":";
            print_string t;
        | Lexer.Whitespace ->
            print_string " ";
        | _ ->
        print_string (Lexer.stringify token)
      end;
      loop stream;
    end with
    | Stream.Failure ->
        if not (end_of_stream stream)
        then raise (Stream.Error "Failed reading stream");
    | Stream.Error err ->
        print_endline err;
  in
  loop stream;
  print_endline "";

  if not ((Stream.peek stream) == None)
  then raise (Stream.Error "Didn't finish reading input");
  *)


  (*
  let argv = Sys.argv in
  let argc = Array.length argv in
  match argc with
  | 1 -> usage ()
  | 2 -> let filename = Array.get argv 1 in
         let contents = readfile filename in
         debug_dump_parse (parse contents)
  | _ -> print_endline "Error: Invalid arguments."
  *)
;;

main ()
