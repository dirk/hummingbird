$(function () {
  // Function from: https://jsfiddle.net/tD6FM/
  var debounce = function (func, threshold, execAsap) {
    var timeout;
    return function debounced () {
      var obj = this, args = arguments;
      function delayed () {
        func.apply(obj, args);
      };

      if (timeout) {
        clearTimeout(timeout);
      } else if (execAsap) {
        func.apply(obj, args);
        return;
      }
      timeout = setTimeout(delayed, threshold || 200);
    };
  }

  var hbInput  = $('.try-source'),
      jsOutput = $('.try-output')

  function doUpdate () {
    var source = hbInput.val(),
        tree   = null
    try {
      tree = hummingbird.parseAndWalk(source)
    } catch(err) {
      var out = err.message
      if (err.name) {
        out = err.name+': '+err.message
      }
      out += "\n"
      if (err.line && err.column) {
        out += "  at line "+err.line+", column "+err.column
      }
      jsOutput.text(out).addClass('is-invalid')
      return
    }
    var output = tree.compile()
    // Update the output and re-highlight
    jsOutput.text(output).removeClass('is-invalid')
    hljs.highlightBlock($('.try-output')[0])
  }

  // Do an initial parse
  doUpdate()
  // Setup a debounced parsing function and run it every time the user
  // keys up
  var debouncedUpdate = debounce(function () {
    doUpdate()
  }, 400)
  hbInput.on('keyup', function () {
    debouncedUpdate()
  })
});

