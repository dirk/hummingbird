var __extends = this.__extends || function (d, b) {
    for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p];
    function __() { this.constructor = d; }
    __.prototype = b.prototype;
    d.prototype = new __();
};
var inherits = require('util').inherits;
// Internal base error class
var BaseError = (function (_super) {
    __extends(BaseError, _super);
    function BaseError(message) {
        _super.call(this, message);
        this.name = 'BaseError';
        this.message = message;
        this.stack = (new Error()).stack;
    }
    return BaseError;
})(Error);
var LocativeError = (function (_super) {
    __extends(LocativeError, _super);
    function LocativeError(message, origin) {
        _super.call(this, message);
        this.name = 'LocativeError';
        this.origin = (origin ? origin : null);
    }
    return LocativeError;
})(BaseError);
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
