var fs       = require('fs'),
    util     = require('util'),
    glob     = require('glob'),
    chalk    = require('chalk')

var paths = {
  typescriptSrc: 'src/**/*.ts'
}

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

var typescript = null

// WriteFileCallback(fileName: string, data: string, writeByteOrderMark: boolean, onError?: (message: string) => void): void;

function logDiagnostics (diagnostics) {
  for (var i = 0; i < diagnostics.length; i++) {
    var diagnostic = diagnostics[i],
        position   = diagnostic.file.getLineAndCharacterOfPosition(diagnostic.start),
        line       = position.line,
        character  = position.character,
        message    = typescript.flattenDiagnosticMessageText(diagnostic.messageText, '\n');
    console.log(diagnostic.file.fileName+' ('+(line + 1)+' ,'+(character + 1)+'): '+message)
  }
}

function isDefinition (fileName) {
  return /\.d\.ts$/.test(fileName)
}

namespace('ts', function () {
  desc('Compile TypeScript source')
  task('compile', function () {
    var compileStart = new Date(),
        startString  = compileStart.toTimeString().split(/\s/)[0];
    console.log('Started at '+chalk.magenta(startString))
    // Load TypeScript if it's not present
    if (!typescript) {
      typescript = require('typescript')
    }
    var files = glob.sync(paths.typescriptSrc)
    // Skip definition files
    files = files.filter(function (name) {
      return !isDefinition(name)
    })
    var program = typescript.createProgram(files, {
      target: typescript.ScriptTarget.ES5,
      module: typescript.ModuleKind.CommonJS,
      sourceMap: true
    })
    logDiagnostics(typescript.getPreEmitDiagnostics(program))

    program.getSourceFiles().forEach(function (sourceFile) {
      var start    = new Date(),
          result   = program.emit(sourceFile),
          fileName = sourceFile.fileName
      // Report diagnostics before logging time
      if (result.diagnostics.length > 0) {
        logDiagnostics(result.diagnostics)
      }
      if (isDefinition(fileName)) { return }

      if (!result.emitSkipped) {
        var durationMs = (new Date() - start)
        console.log("Compiled file '"+chalk.cyan(fileName)+"' in "+chalk.magenta(durationMs+' ms'))
      } else {
        // TODO: Make the message red
        console.log("Failed to compile file '"+chalk.red(fileName)+"'")
      }
    })
    var totalSeconds = (new Date() - compileStart) / 1000,
        formattedSeconds = Math.round(totalSeconds * 100) / 100;
    console.log('Finished in '+chalk.magenta(formattedSeconds+' s'))
  })

  desc('Watch for changes')
  task('watch', function () {
    var chokidar = require('chokidar')
    var watcher = chokidar.watch(paths.typescriptSrc, {
      ignoreInitial: true
    })
    function changed (path) {
      jake.Task['ts:compile'].execute()
    }
    watcher.on('add', changed).on('change', changed)
    watcher.on('ready', function () {
      console.log("Watching for changes in '"+chalk.cyan(paths.typescriptSrc)+"'")
    })
  })
})

//watchTask(['ts:compile'], function () {
//  this.watchFiles.include('./src/**/*.ts')
//})

// vim: filetype=javascript

