var fs       = require('fs'),
    path     = require('path'),
    util     = require('util'),
    child_process = require('child_process'),
    glob     = require('glob'),
    chalk    = require('chalk')

var paths = {
  typescriptSrc: 'src/**/*.ts',
  stdObjs: 'lib/*.o'
}

function exec (cmd, opts) {
  // console.log(cmd)
  var result = child_process.execSync(cmd)
  if (result.length > 0) {
    console.log(result.toString().trim())
  }
}

function formatSeconds (duration) {
  var totalSeconds = duration / 1000;
  return Math.round(totalSeconds * 100) / 100
}

desc('Default build actions')
task('default', ['native:build', 'typescript:compile'])

desc('Compile everything possible')
task('all', ['default', 'specification'])


// Grammar -------------------------------------------------------------------

desc('Build parser from grammar')
task('grammar', ['src/grammar.js'])

file('src/grammar.js', ['src/grammar.pegjs'], function () {
  var start  = new Date(),
      infile = this.prereqs[0]
  exec('node_modules/.bin/pegjs --cache '+infile)
  console.log('Grammar generated in '+chalk.magenta(formatSeconds(new Date() - start)+' s'))
})


// Specification -------------------------------------------------------------

desc('Generate specification tests')
task('specification', function () {
  var start        = new Date(),
      parseSpec    = require('./src/spec-parser').parseSpecification
      specSource   = fs.readFileSync(__dirname+'/doc/specification.md').toString(),
      runnerSource = fs.readFileSync(__dirname+'/share/spec-runner.js').toString(),
      specs        = parseSpec(specSource),
      specTestDir  = __dirname+'/test/spec'

  // Remove old specification files
  var files   = fs.readdirSync(specTestDir),
      removed = 0
  for (var i = 0; i < files.length; i++) {
    var f = path.join(specTestDir, files[i])
    if (!/\.js$/.test(f) && !/\.hb$/.test(f)) { continue }
    fs.unlinkSync(f)
    removed += 1
  }
  console.log('Removed '+chalk.magenta(removed)+' old specification files')

  // Now generate the specs
  for (var i = specs.length - 1; i >= 0; i--) {
    var spec = specs[i]

    var js = spec.js+"\n",
        hb = spec.hb+"\n"
    fs.writeFileSync(specTestDir+'/source-'+spec.name+'.js', js)
    fs.writeFileSync(specTestDir+'/source-'+spec.name+'.hb', hb)

    var runner = runnerSource.replace(/NAME/g, spec.name)
    fs.writeFileSync(specTestDir+'/test-'+spec.name+'.js', runner)
    
    console.log("Generated tests for '"+chalk.cyan(spec.name)+"'")
  }
  console.log('Specification tests generated in '+chalk.magenta(formatSeconds(new Date() - start)+' s'))
})


// Native LLVM target --------------------------------------------------------

namespace('native', function () {
  desc('Build the standard library')
  task('build', ['lib/std.o'])

  file('lib/std.o', ['ext/std.c'], function () {
    var start   = new Date(),
        outfile = this.name,
        infile  = this.prereqs[0]
    exec("clang -c "+infile+" -o "+outfile)
    console.log('Native library compiled in '+chalk.magenta(formatSeconds(new Date() - start)+' s'))
  })

  desc('Clean build artifacts')
  task('clean', function () {
    var files = glob.sync(paths.stdObjs)
    for (var i = 0; i < files.length; i++) {
      var file = files[i]
      fs.unlinkSync(file)
    }
  })
})


// TypeScript ----------------------------------------------------------------

var typescript = null

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

namespace('typescript', function () {
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
        console.log("Failed to compile file '"+chalk.red(fileName)+"'")
      }
    })
    console.log('Finished in '+chalk.magenta(formatSeconds(new Date() - compileStart)+' s'))
  })

  desc('Watch for changes')
  task('watch', function () {
    var chokidar = require('chokidar')
    var watcher = chokidar.watch(paths.typescriptSrc, {
      ignoreInitial: true
    })
    function changed (path) {
      jake.Task['typescript:compile'].execute()
    }
    watcher.on('add', changed).on('change', changed)
    watcher.on('ready', function () {
      console.log("Watching for changes in '"+chalk.cyan(paths.typescriptSrc)+"'")
    })
  })
})

// watchTask(['typescript:compile'], function () {
//   this.watchFiles.include('./src/**/*.ts')
// })

// vim: filetype=javascript

