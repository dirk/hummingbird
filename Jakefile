

var exec = function (cmd, opts, cb) {
  if (typeof cmd === 'string') { cmd = [cmd] }
  if (!opts) { opts = {} }
  opts.printStdout = true
  opts.printStderr = true
  // Print the commands we're going to run
  cmd.forEach(function (c) {
    console.log(c)
  })
  jake.exec(cmd, opts, cb)
}

desc('Build the standard library')
file('lib/std.o', ['ext/std.c'], function (a, b, c) {
  var outfile = this.name,
      infile  = this.prereqs[0]
  exec("clang -c ext/std.c -o lib/std.o")
})

desc('Default building actions')
task('default', ['lib/std.o'])

// vim: filetype=javascript

