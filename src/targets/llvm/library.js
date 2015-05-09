var fs        = require('fs'),
    path      = require('path'),
    llvm2Path = path.resolve(__dirname+'/../../../../llvm2')

// Look for llvm2 as a sibling of the top-level Hummingbird directory
if (!fs.existsSync(llvm2Path)) {
  llvm2Path = 'llvm2'
}

// Expose the correct instance of the llvm2 library
module.exports = require(llvm2Path)

