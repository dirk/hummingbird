
var gulp        = require('gulp'),
    gtypescript = require('gulp-typescript'),
    gutil       = require('gulp-util'),
    watch       = require('gulp-watch'),
    chalk       = require('chalk'),
    path        = require('path'),
    through     = require('through2'),
    typescript  = require('typescript'),
    cwd         = process.cwd()

var paths = {
  typescriptSrc: 'src/**/*.ts'
}

gulp.task('default', function () {
  var fileLogger = through.obj(function (file, encoding, callback) {
    var p = path.relative(cwd, file.path)
    gutil.log("Wrote file '"+chalk.cyan(p)+"'");
    callback(null, file)
  })

  return gulp.src(paths.typescriptSrc)
  .pipe(gtypescript({
    // TypeScript opts
    typescript: typescript,
    module:     'commonjs'
  }))
  .pipe(gulp.dest('src'))
  .pipe(fileLogger)
})

gulp.task('watch', function () {
  watch(paths.typescriptSrc, function () {
    console.log('there')
    gulp.start('default')
  })
})

