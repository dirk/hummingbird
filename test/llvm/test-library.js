var helper = require('./helper'),
    fs     = require('fs'),
    path   = require('path'),
    expect = require('expect.js')

var LLVM   = require('../../src/targets/llvm/library'),
    target = require('../../src/targets/llvm/target')

describe('LLVM library', function () {
  describe('target information', function () {
    var ctx
    it('should set up a module', function () {
      ctx = {
        module: {ptr: LLVM.Library.LLVMModuleCreateWithName('test')},
        logger: {info: function () {}}
      }
      expect(ctx.module.ptr.isNull()).to.be(false)
    })
    it('should initialize', function () {
      target.initializeTarget(ctx)
    })
    it('should have target data', function () {
      var td = ctx.targetData
      expect(td).to.be.a(Buffer)
      expect(td.isNull()).to.be(false)
    })
  })
})

