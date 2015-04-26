var __extends = this.__extends || function (d, b) {
    for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p];
    function __() { this.constructor = d; }
    __.prototype = b.prototype;
    d.prototype = new __();
};
var inherits = require('util').inherits;
var LocativeError = (function (_super) {
    __extends(LocativeError, _super);
    function LocativeError(message, origin) {
        _super.call(this);
        this.message = message;
        this.origin = (origin ? origin : null);
    }
    return LocativeError;
})(Error);
var InternalCompilerError = (function (_super) {
    __extends(InternalCompilerError, _super);
    function InternalCompilerError(message, origin) {
        _super.call(this, message, origin);
        this.name = 'InternalCompilerError';
    }
    return InternalCompilerError;
})(LocativeError);
exports.InternalCompilerError = InternalCompilerError;
var TypeError = (function (_super) {
    __extends(TypeError, _super);
    function TypeError(message, origin) {
        _super.call(this, message, origin);
        this.name = 'TypeError';
    }
    return TypeError;
})(LocativeError);
exports.TypeError = TypeError;
