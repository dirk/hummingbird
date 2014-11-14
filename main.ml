#require "core"

open Core

let readfile filename = In_channel.read_all filename

let parse contents = contents

let debug_dump_parse tree =
  print_endline "Hello world!"

let usage () =
  print_endline "Usage: hb file"

let main () =
  let argv = Sys.argv in
  let argc = Array.length argv in
  match argc with
  | 1 -> usage ()
  | 2 -> let filename = Array.get argv 1 in
         let contents = readfile filename in
         debug_dump_parse (parse contents)
  | _ -> print_endline "Error: Invalid arguments."
;;

main ()
