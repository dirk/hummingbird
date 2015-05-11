#!/usr/bin/env node --expose_gc

// Load `jake` global
require('jake');
var args = process.argv.slice(2);
jake.run.apply(jake, args);

