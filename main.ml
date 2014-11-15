
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
  let stream = Lexer.lex (Stream.of_string "for a TEST") in
  let rec loop stream =
    try begin
      let token = Stream.next stream in
      begin
        match token with
        | Lexer.Word t ->
            print_string "w(";
            print_string t;
            print_string ")";
        | Lexer.Keyword kw ->
            begin
              match kw with
              | Lexer.For -> print_string "kw(for)";
            end;
        | Lexer.Whitespace ->
            print_string " ";
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

  if not ((Stream.peek stream) == None)
  then raise (Stream.Error "Didn't finish reading input");
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
