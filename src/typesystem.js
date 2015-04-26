/// <reference path="typescript/node-0.12.0.d.ts" />
var util = require('util');
var errors = require('./errors');
var AST = require('./ast');
var inherits = util.inherits, inspect = util.inspect, types = require('./types'), scope = require('./typesystem/scope'), Scope = scope.Scope, ClosingScope = scope.ClosingScope, TypeError = errors.TypeError;
function TypeSystem() {
    this.root = new Scope();
    this.root.isRoot = true;
    this.bootstrap();
    // File and compiler will be null when not actively walking a tree
    this.file = null;
    this.compiler = null;
}
// Add the bootstrap methods to the TypeSystem
require('./typesystem/bootstrap')(TypeSystem);
TypeSystem.prototype.findByName = function (name) {
    if (typeof name !== 'string') {
        throw new Error('Non-string name for type lookup');
    }
    return this.root.getLocal(name);
};
function assertInstanceOf(value, type, msg) {
    if (value instanceof type) {
        return;
    }
    if (!msg) {
        msg = 'Incorrect type; expected ' + type.name + ', got ' + value.constructor.name;
    }
    throw new Error(msg);
}
// AST typing -----------------------------------------------------------------
TypeSystem.prototype.walk = function (rootNode, file, compiler) {
    assertInstanceOf(rootNode, AST.Root, "Node must be root");
    this.file = file ? file : null;
    this.compiler = compiler ? compiler : null;
    var topLevelScope = new Scope(this.root);
    // Save this top-level scope on the root
    rootNode.scope = topLevelScope;
    var self = this;
    rootNode.statements.forEach(function (stmt) {
        self.visitStatement(stmt, topLevelScope, rootNode);
    });
    // Reset the compiler property now that we're done walking
    this.file = null;
    this.compiler = null;
};
TypeSystem.prototype.visitBlock = function (node, scope) {
    if (node.scope) {
        throw new TypeError('Scope already established for block', node);
    }
    // Save the scope for this block for later use by target compilers
    node.scope = scope;
    var self = this;
    node.statements.forEach(function (stmt) {
        self.visitStatement(stmt, scope, node);
    });
};
TypeSystem.prototype.visitStatement = function (node, scope, parentNode) {
    switch (node.constructor) {
        case AST.Assignment:
            if (node.lvalue instanceof AST.Let) {
                this.visitLet(node, scope);
            }
            else if (node.lvalue instanceof AST.Var) {
                this.visitVar(node, scope);
            }
            else if (node.lvalue instanceof AST.Path) {
                this.visitPath(node, scope);
            }
            else {
                throw new TypeError('Cannot visit Assignment with: ' + node.lvalue + ' (' + node.lvalue.constructor.name + ')');
            }
            break;
        case AST.If:
            this.visitIf(node, scope);
            break;
        case AST.While:
            this.visitWhile(node, scope);
            break;
        case AST.For:
            this.visitFor(node, scope);
            break;
        case AST.Return:
            this.visitReturn(node, scope, parentNode);
            break;
        case AST.Binary:
            if (node.isBinaryStatement()) {
                this.visitBinary(node, scope);
            }
            else {
                throw new TypeError('Cannot visit non-statement binary: ' + node.op);
            }
            break;
        // case AST.Chain:
        //   this.visitChain(node, scope)
        //   break
        case AST.Multi:
            this.visitMulti(node, scope);
            break;
        case AST._Function:
            // Create the searcher in this parent node
            // TODO: Maybe just pass along the parent node rather than generating
            //       a whole new anonymous function every time we encounter a
            //       function statement?
            var searchInParent = function (cb) {
                var statements = parentNode.statements, found = null;
                // Call `cb` on each statement of the parent until it returns true
                for (var i = statements.length - 1; i >= 0; i--) {
                    var stmt = statements[i], ret = cb(stmt);
                    if (ret === true) {
                        found = stmt;
                        break;
                    }
                }
                return found;
            };
            this.visitFunctionStatement(node, scope, searchInParent);
            break;
        case AST.Class:
            this.visitClass(node, scope);
            break;
        case AST.Import:
            this.visitImport(node, scope, parentNode);
            break;
        case AST.Export:
            this.visitExport(node, scope, parentNode);
            break;
        case AST.Property:
            this.visitProperty(node, scope, parentNode);
            break;
        case AST.Call:
            this.visitCall(node, scope);
            break;
        default:
            throw new TypeError("Don't know how to visit: " + node.constructor.name, node);
            break;
    }
};
TypeSystem.prototype.visitImport = function (node, scope, parentNode) {
    assertInstanceOf(node, AST.Import);
    assertInstanceOf(node.name, String, "Import expects String as path");
    assertInstanceOf(parentNode, AST.Root, "Import can only be a child of a Root");
    if (!this.compiler) {
        throw new Error('Type-system not provided with current Compiler instance');
    }
    if (!this.file) {
        throw new Error('Type-system not provided with current File instance');
    }
    // Add ourselves to the root's list of imports it contains
    parentNode.imports.push(node);
    var moduleName = node.name;
    // Preserve current file to restore after visiting the imported file
    var currentFile = this.file;
    // Now ask the compiler to import the file
    var importedFile = this.compiler.importFileByName(moduleName);
    node.file = importedFile;
    // Restore the current file and push the imported file as a dependency of it
    this.file = currentFile;
    this.file.dependencies.push(importedFile);
    // Then build a module object for it
    var module = new types.Module();
    module.name = moduleName;
    var exportedNames = Object.keys(importedFile.exports);
    for (var i = exportedNames.length - 1; i >= 0; i--) {
        var name = exportedNames[i], type = importedFile.exports[name];
        // Add the exported name-type pair to the module and set it as a
        // read-only property
        module.setTypeOfProperty(name, type);
        module.setFlagsOfProperty(name, 'r');
    }
    if (node.using) {
        assertInstanceOf(node.using, Array);
        for (var i = 0; i < node.using.length; i++) {
            var use = node.using[i], useType = module.getTypeOfProperty(use);
            scope.setLocal(use, new types.Instance(useType));
        }
    }
    else {
        // If there's no `using` then just add the whole module
        scope.setLocal(moduleName, module);
    }
    // Now create a faux instance of this module and add it to the scope
    // scope.setLocal(moduleName, new types.Instance(module))
};
TypeSystem.prototype.visitExport = function (node, scope, parentNode) {
    // Make sure our parent node is the root
    assertInstanceOf(parentNode, AST.Root, "Import can only be a child of a Root");
    // Add ourselves to the root node's list of export nodes
    parentNode.exports.push(node);
    var name = node.name;
    // Make sure we're in the top-level scope
    if (scope.parent !== this.root) {
        throw new TypeError('Exporting from non-root scope', node);
    }
    // Look up the type for the name in the root
    var type = scope.getLocal(name);
    this.file.module.setTypeOfProperty(name, type);
    // Need to unbox an instance if we encounter one
    if (type instanceof types.Instance) {
        type = type.type;
    }
    node.type = type;
    // TODO: Check that the name is a constant binding (rather than variable)
    this.file.exports[name] = type;
};
TypeSystem.prototype.visitClass = function (node, scope) {
    var rootObject = this.rootObject;
    // Create a new Object type with the root object as the supertype
    var klass = new types.Object(rootObject);
    klass.name = node.name;
    scope.setLocal(klass.name, klass);
    scope.setFlagsForLocal(klass.name, Scope.Flags.Constant);
    // Now create a new scope and visit the definition in that scope
    var scope = new Scope(scope);
    this.visitClassDefinition(node.definition, scope, klass);
    // Set the class as the node's type
    node.type = klass;
};
// Given a class type and a scope, sets up `this` bindings in that scope
// for instances of that class (with proper constant flags)
function setupThisInScope(klass, scope) {
    scope.setLocal('this', new types.Instance(klass));
    scope.setFlagsForLocal('this', Scope.Flags.Constant);
}
TypeSystem.prototype.visitClassDefinition = function (node, scope, klass) {
    var self = this;
    node.statements.forEach(function (stmt) {
        switch (stmt.constructor) {
            case AST.Assignment:
                if (stmt.type !== 'var' && stmt.type !== 'let') {
                    throw new TypeError('Unexpected assignment type: ' + stmt.type, stmt);
                }
                var propertyName = stmt.lvalue.name;
                // Check that there's a type specified for this slot
                if (!stmt.lvalue.immediateType) {
                    throw new TypeError('Missing type for class slot: ' + propertyName);
                }
                var propertyType = self.resolveType(stmt.lvalue.immediateType, scope);
                // Check that the default (rvalue) is constant if present
                // TODO: Smarter checking of constant-ness of default values when it's "let"
                if (stmt.rvalue && !(stmt.rvalue instanceof AST.Literal)) {
                    throw new TypeError('Cannot handle non-literal default for property: ' + propertyName);
                }
                // Create the property on the object with the resolved type
                klass.setTypeOfProperty(propertyName, propertyType);
                // Add read-only flags for this property when the assignment .type is "let"
                if (stmt.type === 'let') {
                    klass.setFlagsOfProperty(propertyName, 'r');
                }
                break;
            case AST._Function:
                self.visitClassFunction(stmt, scope, klass);
                break;
            case AST.Init:
                var initType = new types.Function(self.rootObject), initScope = new Scope(scope);
                // Add an instance of 'this' for the initializer's scope
                setupThisInScope(klass, initScope);
                // Resolve the arguments
                var args = [];
                stmt.args.forEach(function (arg) {
                    var type = self.resolveType(arg.type);
                    initScope.setLocal(arg.name, new types.Instance(type));
                    args.push(type);
                });
                initType.args = args;
                initType.ret = self.root.getLocal('Void');
                // Then visit the block with the new scope
                self.visitBlock(stmt.block, initScope);
                // Add the Function init type to the class and to this initializer node
                klass.addInitializer(initType);
                stmt.type = initType;
                break;
            default:
                throw new TypeError("Don't know how to visit '" + stmt.constructor.name + "' in class definition");
                break;
        }
    });
};
TypeSystem.prototype.visitClassFunction = function (node, scope, klass) {
    var functionName = node.name;
    // Check that it's a function statement (ie. has a name)
    if (!functionName) {
        throw new TypeError('Missing function name', node);
    }
    // Run the generic visitor to figure out argument and return types
    this.visitFunction(node, scope, function (functionType, functionScope) {
        assertInstanceOf(functionScope, ClosingScope, "Function's scope must be a ClosingScope");
        setupThisInScope(klass, functionScope);
    });
    var functionInstance = node.type;
    // Unbox the instance generated by the visitor to get the pure
    // function type
    var functionType = functionInstance.type;
    // Let the function type know that it's an instance method (used by the compiler)
    functionType.isInstanceMethod = true;
    // Add that function type as a property of the class
    // TODO: Maybe have a separate dictionary for instance methods
    klass.setTypeOfProperty(functionName, functionType);
};
TypeSystem.prototype.visitFor = function (node, scope) {
    this.visitStatement(node.init, scope);
    // If there's a condition present then we need to visit the expression
    // and type-check what it resolves to
    if (node.cond) {
        this.visitExpression(node.cond, scope);
        var condType = node.cond.type;
        if (!condType) {
            throw new TypeError('Missing type of `for` condition', node.cond);
        }
        // Check that the condition resolves to a boolean
        if (!condType.equals(this.findByName('Boolean'))) {
            throw new TypeError('Expected `for` condition to resolve to a Boolean', node.cond);
        }
    }
    this.visitStatement(node.after, scope);
    var blockScope = new Scope(scope);
    this.visitBlock(node.block, blockScope);
};
TypeSystem.prototype.visitIf = function (node, scope) {
    assertInstanceOf(node.block, AST.Block, 'Expected Block in If statement');
    this.visitExpression(node.cond, scope);
    // Handle the main if block
    var blockScope = new Scope(scope);
    this.visitBlock(node.block, blockScope);
    // Visit each of the else-ifs
    if (node.elseIfs) {
        for (var i = 0; i < node.elseIfs.length; i++) {
            var elseIf = node.elseIfs[i], elseIfBlockScope = new Scope(scope);
            this.visitExpression(elseIf.cond, scope);
            this.visitBlock(elseIf.block, elseIfBlockScope);
        }
    }
    // Handle the else block if present
    if (node.elseBlock) {
        var elseBlockScope = new Scope(scope);
        this.visitBlock(node.elseBlock, elseBlockScope);
    }
};
TypeSystem.prototype.visitWhile = function (node, scope) {
    assertInstanceOf(node.block, AST.Block, 'Expected Block in While statement');
    this.visitExpression(node.expr, scope);
    var blockScope = new Scope(scope);
    this.visitBlock(node.block, blockScope);
};
TypeSystem.prototype.visitReturn = function (node, scope, parentNode) {
    if (node.expr === undefined) {
        throw new TypeError('Cannot handle undefined expression in Return');
    }
    var exprType = null;
    if (node.expr === null) {
        var voidType = this.root.getLocal('Void');
        exprType = new types.Instance(voidType);
    }
    else {
        var expr = node.expr;
        exprType = this.resolveExpression(expr, scope);
    }
    node.type = exprType;
    // Handle the parent block if present
    if (parentNode) {
        if (!((parentNode instanceof AST.Block) || (parentNode instanceof AST.Root))) {
            throw new TypeError('Expected Block or Root as parent of Return', node);
        }
        // assertInstanceOf(parentNode, AST.Block, 'Expected Block as parent of Return')
        if (parentNode.returnType) {
            throw new TypeError('Block already has returned');
        }
        // The expression should return an instance, we'll have to unbox that
        assertInstanceOf(exprType, types.Instance, 'Expected Instance as argument to Return');
        parentNode.returnType = exprType.type;
    }
};
TypeSystem.prototype.visitPath = function (node, scope) {
    var path = node.lvalue;
    var foundScope = scope.findScopeForName(path.name);
    if (foundScope === null) {
        throw new TypeError('Failed to find ' + path.name);
    }
    var lvalueType = foundScope.get(path.name);
    // Now revise that type according to the path
    path.path.forEach(function (item) {
        switch (item.constructor) {
            case AST.Identifier:
                if (!(lvalueType instanceof types.Instance)) {
                    throw new TypeError('Cannot get property of non-Instance', item);
                }
                var propertyName = item.name;
                // Unbox the lvalue instance
                var instance = lvalueType, type = instance.type;
                // Finally look up the type of the property and box it up
                var newType = type.getTypeOfProperty(propertyName, item);
                lvalueType = new types.Instance(newType);
                // Also check the flags to make sure we're not trying to write to
                // a read-only property
                if (type.hasPropertyFlag(propertyName, types.Flags.ReadOnly)) {
                    throw new TypeError('Trying to assign to read-only property: ' + propertyName, node);
                }
                // Set the type that is going to be returned at this stage on the item
                item.type = lvalueType;
                break;
            default:
                throw new TypeError('Cannot handle item in path of type: ' + item.constructor.name, node);
        }
    });
    var rvalueType = this.resolveExpression(node.rvalue, scope);
    if (!lvalueType.equals(rvalueType)) {
        throw new TypeError('Unequal types in assignment: ' + lvalueType.inspect() + ' </> ' + rvalueType.inspect(), node);
    }
};
TypeSystem.prototype.resolveType = function (node, scope) {
    var self = this;
    switch (node.constructor) {
        case AST.FunctionType:
            var args = node.args.map(function (arg) { return self.resolveType(arg, scope); }), ret = this.resolveType(node.ret, scope);
            // Build the type and return it
            return new types.Function(this.rootObject, args, ret);
        case AST.NameType:
            // TODO: Improve the handling and look-ups of these; right now they're way too naive
            return this.findByName(node.name);
        default:
            throw new Error("Can't walk: " + node.constructor.name);
    }
};
TypeSystem.prototype.visitLet = function (node, scope) {
    var lvalueType = new types.Unknown();
    var name = node.lvalue.name;
    // If we have an explicit type then look it up
    if (node.lvalue.immediateType) {
        var immediateTypeNode = node.lvalue.immediateType;
        // lvalueType = this.findByName(...)
        lvalueType = this.resolveType(immediateTypeNode, scope);
        // Box the type into an instance
        lvalueType = new types.Instance(lvalueType);
    }
    // Create a scope inside the Let statement for recursive calls
    var letScope = new Scope(scope);
    letScope.setLocal(name, lvalueType);
    if (node.rvalue) {
        // rvalue is an expression so let's determine its type first.
        var rvalueType = this.resolveExpression(node.rvalue, letScope, function (immediateType) {
            if (lvalueType instanceof types.Unknown) {
                // If the lvalue is unknown then annotate it with the resolved type
                lvalueType.known = new types.Instance(immediateType);
            }
        });
        if (lvalueType instanceof types.Unknown) {
            // If the lvalue was inferred then update on the lvalue
            node.lvalue.type = rvalueType;
            scope.setLocal(name, rvalueType);
        }
        else {
            // If the lvalue type is explicit then make sure they match up
            if (!lvalueType.equals(rvalueType)) {
                var message = 'Unequal types in declaration: ' + lvalueType.inspect() + ' </> ' + rvalueType.inspect();
                throw new TypeError(message, node);
            }
            scope.setLocal(name, lvalueType);
        }
    }
    else {
        // No rvalue present
        node.lvalue.type = lvalueType;
        scope.setLocal(name, lvalueType);
    }
    // Now that the local is set in the parent scope we can set its flags
    // if it was a `let`-declaration
    if (node.type === 'let') {
        scope.setFlagsForLocal(name, Scope.Flags.Constant);
    }
};
// Alias the var visitor to the let visitor
TypeSystem.prototype.visitVar = TypeSystem.prototype.visitLet;
TypeSystem.prototype.resolveExpression = function (expr, scope, immediate) {
    // If we've already deduced the type of this then just return it
    if (expr.type) {
        return expr.type;
    }
    this.visitExpression(expr, scope, immediate);
    if (expr.type === null || expr.type === undefined) {
        throw new TypeError('Failed to resolve type');
    }
    return expr.type;
};
TypeSystem.prototype.visitExpression = function (node, scope, immediate) {
    switch (node.constructor) {
        case AST._Function:
            // Sanity checks to make sure the name and when are not present
            if (node.name) {
                throw new TypeError('Function expression cannot have a `name`', node);
            }
            if (node.when) {
                throw new TypeError('Function expression cannot have a `when` condition', node);
            }
            // Then run the visitor
            this.visitFunction(node, scope, immediate);
            break;
        case AST.Binary:
            this.visitBinary(node, scope);
            break;
        // case AST.Chain:
        //   this.visitChain(node, scope)
        //   break
        case AST.Literal:
            this.visitLiteral(node, scope);
            break;
        case AST.New:
            this.visitNew(node, scope);
            break;
        case AST.Identifier:
            this.visitIdentifier(node, scope);
            break;
        case AST.Property:
            this.visitProperty(node, scope);
            break;
        case AST.Call:
            this.visitCall(node, scope);
            break;
        default:
            throw new Error("Can't visit expression: " + node.constructor.name);
    }
};
TypeSystem.prototype.visitLiteral = function (node, scope) {
    // If we've already identified the type
    if (node.type) {
        return node.type;
    }
    else if (node.typeName) {
        var type = this.findByName(node.typeName);
        node.type = new types.Instance(type);
        return type;
    }
    else {
        throw new TypeError('Unknown literal type: ' + node.typeName);
    }
};
TypeSystem.prototype.visitNew = function (node, scope) {
    // Look up the type of what we're going to construct
    var type = scope.get(node.name);
    node.constructorType = type;
    // Construct an instance of that type
    var instance = new types.Instance(type);
    node.type = instance;
    if (type.initializers.length === 0) {
        throw new TypeError('No initializer found for class', node);
    }
    // Unboxed types of all the arguments for comparing with the class'
    // set of initializers.
    var argTypes = new Array(node.args.length);
    // Visit all of the arguments
    for (var i = 0; i < node.args.length; i++) {
        var arg = node.args[i];
        this.visitExpression(arg, scope);
        var argType = arg.type;
        if (!(argType instanceof types.Instance)) {
            throw new TypeError('Expected Instance as argument to New');
        }
        argTypes[i] = argType.type;
    }
    var initializers = type.initializers, initializer = false;
    // Look for a matching initializer
    for (var i = initializers.length - 1; i >= 0; i--) {
        var init = initializers[i];
        var argsMatch = init.argsMatch(argTypes);
        if (argsMatch) {
            initializer = init;
            break;
        }
    }
    if (initializer === false) {
        throw new TypeError('No initializer not found');
    }
    node.setInitializer(initializer);
};
var COMPARATOR_OPS = ['<'];
TypeSystem.prototype.visitBinary = function (node, scope) {
    var lexprType = this.resolveExpression(node.lexpr, scope);
    var rexprType = this.resolveExpression(node.rexpr, scope);
    assertInstanceOf(lexprType, types.Instance, 'Expected Instance in L-value');
    assertInstanceOf(rexprType, types.Instance, 'Expected Instance in R-value');
    if (lexprType.equals(rexprType)) {
        // Naive type assignment based off left side; this is refined below
        node.type = lexprType;
    }
    else {
        throw new TypeError('Unequal types in binary operation: ' + lexprType.inspect() + ' </> ' + rexprType.inspect());
    }
    // TODO: Check adder, comparator, etc. interfaces of the left and right
    var op = node.op;
    if (COMPARATOR_OPS.indexOf(op) !== -1) {
        node.type = this.findByName('Boolean');
    }
};
function getAllReturnTypes(block) {
    var returnTypes = [];
    if (block.returnType) {
        returnTypes.push(block.returnType);
    }
    block.statements.forEach(function (stmt) {
        var types = null;
        switch (stmt.constructor) {
            case AST.If:
                types = getAllReturnTypes(stmt.block);
                if (stmt.elseBlock) {
                    types = types.concat(getAllReturnTypes(stmt.elseBlock));
                }
                returnTypes = returnTypes.concat(types);
                break;
            case AST.While:
            case AST.For:
                types = getAllReturnTypes(stmt.block);
                returnTypes = returnTypes.concat(types);
                break;
        }
    });
    return returnTypes;
}
TypeSystem.prototype.visitFunction = function (node, parentScope, immediate) {
    if (node.type) {
        return node.type;
    }
    var self = this;
    var type = new types.Function(this.rootObject);
    // Set the type of this node to an instance of the function type
    node.type = new types.Instance(type);
    if (node.ret) {
        type.ret = this.resolveType(node.ret);
    }
    // Set up a closing scope for everything in the function
    var functionScope = new ClosingScope(parentScope);
    // Save this new scope on the node object for later use
    node.scope = functionScope;
    // If we have a callback for the immediate (not-yet-fully resolved type)
    // then call it now. This is also an opportunity for class and instance
    // methods to add their `this` bindings to the function's closing scope.
    if (immediate !== undefined) {
        immediate(type, functionScope);
    }
    // Build up the args to go into the type definition
    var typeArgs = [], n = 0;
    node.args.forEach(function (arg) {
        // Deprecated simplistic type lookup:
        //   var argType = self.findByName(arg.type)
        if (!arg.type) {
            throw new TypeError('Missing type for argument ' + n, node);
        }
        var argType = self.resolveType(arg.type);
        // Setup a local Instance in the function's scope for the argument
        functionScope.setLocal(arg.name, new types.Instance(argType));
        // Add the type to the type's args
        typeArgs.push(argType);
        n += 1;
    });
    type.args = typeArgs;
    // Begin by visiting our block
    this.visitBlock(node.block, functionScope);
    // Get all possible return types of this function (recursively collects
    // returning child blocks).
    var returnTypes = getAllReturnTypes(node.block);
    // If there is a declared return type then we need to check that all the found
    // returns match that type
    if (type.ret) {
        returnTypes.forEach(function (returnType) {
            if (!type.ret.equals(returnType)) {
                throw new TypeError('Type returned by function does not match declared return type');
            }
        });
        return;
    }
    // Otherwise we need to try to unify the returns; this could potentially be
    // a very expensive operation, so we'll warn the user if they do too many
    if (returnTypes.length > 4) {
        var returns = returnTypes.length, file = node._file, line = node._line, warning = "Warning: Encountered " + returns + " return statements in function\n" +
            "  Computing type unions can be expensive and should be used carefully!\n" +
            "  at " + file + ":" + line + "\n";
        process.stderr.write(warning);
    }
    // Slow quadratic uniqueness checking to reduce the set of return types
    // to distinct ones
    var reducedTypes = uniqueWithComparator(returnTypes, function (a, b) {
        return a.equals(b);
    });
    if (reducedTypes.length > 1) {
        var t = reducedTypes.map(function (t) { return t.inspect(); }).join(', ');
        throw new TypeError('Too many return types (have ' + t + ')', node);
    }
    // Final return type
    var returnType, isReturningVoid = false;
    if (reducedTypes.length > 0) {
        returnType = reducedTypes[0];
    }
    else {
        isReturningVoid = true;
        returnType = this.root.getLocal('Void');
    }
    // Update the type definition (if there we 0 then it will be null which is
    // Void in the type-system)
    type.ret = returnType;
    // If we know we're returning Void then check for a missing final return
    // and insert it to help the user out.
    if (isReturningVoid) {
        var lastStatement = node.block.statements[node.block.statements.length - 1];
        if (lastStatement && !(lastStatement instanceof AST.Return)) {
            // Last statement isn't a return, so let's insert one for them
            var returnStmt = new AST.Return(null);
            returnStmt.setPosition('(internal)', -1, -1);
            node.block.statements.push(returnStmt);
            this.visitReturn(returnStmt, functionScope, node.block);
            // Update the `isLastStatement` properties
            lastStatement.isLastStatement = false;
            returnStmt.isLastStatement = true;
        }
    }
}; //visitFunction
function uniqueWithComparator(array, comparator) {
    var acc = [], length = array.length;
    for (var i = 0; i < length; i++) {
        for (var j = i + 1; j < length; j++) {
            var a = array[i], b = array[j];
            if (comparator(a, b)) {
                j = ++i;
            }
        }
        acc.push(array[i]);
    }
    return acc;
}
TypeSystem.prototype.visitMultiFunction = function (node, scope, multiNode) {
    var multiType = multiNode.type;
    // Add this implementation to its list of functions and set the parent of
    // the function so that it knows not to codegen itself
    multiType.addFunctionNode(node);
    node.setParentMultiType(multiNode.type);
    // Fill out any missing types
    for (var i = 0; i < node.args.length; i++) {
        var arg = node.args[i];
        // Type is specified so we don't need to worry about it
        if (arg.type) {
            continue;
        }
        // Set the argument's type to the multi argument's type
        arg.type = multiNode.args[i].type;
    }
    // First run the generic function visitor
    this.visitFunction(node, scope);
    // Type-system checks
    if (typeof node.name !== 'string') {
        throw new TypeError('Non-string name for function statement', node);
    }
    assertInstanceOf(node.scope, Scope, "Missing function's scope");
    // Now do statement-level visiting
    if (node.when) {
        this.visitExpression(node.when, node.scope);
    }
};
TypeSystem.prototype.visitNamedFunction = function (node, scope) {
    this.visitFunction(node, scope);
    scope.setLocal(node.name, node.type);
};
TypeSystem.prototype.visitFunctionStatement = function (node, scope, searchInParent) {
    var name = node.name;
    // Now look up the parent `multi` in the containing block
    var multiNode = searchInParent(function (stmt) {
        if (stmt.constructor === AST.Multi && stmt.name === name) {
            return true;
        }
        return false;
    });
    if (multiNode) {
        this.visitMultiFunction(node, scope, multiNode);
    }
    else {
        this.visitNamedFunction(node, scope);
    }
};
// Resolve an Unknown type to a known one (sort of a second pass) or throw
// an error if it's still unknown
var know = function (node, type) {
    if (type instanceof types.Unknown) {
        if (type.known === null) {
            throw new TypeError('Unknown type');
        }
        return type.known;
    }
    return type;
};
function parentTypeLooup(node, scope, name) {
    if (node.parent === null) {
        return scope.get(name);
    }
    else {
        return node.parent.startingType;
    }
}
TypeSystem.prototype.visitIdentifier = function (node, scope) {
    if (node.parent) {
        // throw new TypeError("Identifier cannot have a parent", node)
        node.type = this.getTypeOfTypesProperty(node.parent.baseType, node.name);
    }
    else {
        node.type = scope.get(node.name);
    }
};
TypeSystem.prototype.visitProperty = function (node, scope, parentNode) {
    var property = node.property, base = node.base;
    // Set up the parents
    if (node.parent) {
        base.parent = node.parent;
    }
    property.parent = node;
    // Then visit the base and the child
    this.visitExpression(node.base, scope);
    node.baseType = node.base.type;
    if (typeof property === 'string') {
        throw new Error('Unreachable');
        // If it's just basic string then look up the property on ourselves
        var propertyType = this.getTypeOfTypesProperty(node.baseType, property);
        node.type = propertyType;
    }
    else {
        // Otherwise visit the property as a full expression
        this.visitExpression(property, scope);
        // Update from the child's type
        node.type = property.type;
    }
};
TypeSystem.prototype.visitCall = function (node, scope) {
    // Make our base identifier point to our parent so it will resolve correctly
    // when we visit it
    node.base.parent = node.parent;
    this.visitExpression(node.base, scope);
    node.baseType = node.base.type;
    assertInstanceOf(node.baseType, types.Instance, 'Expected Instance for base type of Call');
    var functionType = node.baseType.type;
    assertInstanceOf(functionType, types.Function, 'Expected Function for unboxed type');
    var args = node.args, typeArgs = functionType.args;
    // Basic length check
    if (args.length !== typeArgs.length) {
        throw new TypeError("Argument length mismatch, expected: " + typeArgs.length + ", got: " + args.length);
    }
    // Item-wise compare the arguments (given) with the parameters (expected)
    for (var i = 0; i < typeArgs.length; i++) {
        var arg = args[i];
        this.visitExpression(arg, scope);
        // Get the type of the argument (from the caller) and the parameter (from
        // the function's definition).
        var argTy = arg.type, parTy = typeArgs[i];
        assertInstanceOf(argTy, types.Instance, "Expected Instance as function argument, got: " + argTy.inspect);
        // Unbox the instance
        argTy = argTy.type;
        if (!parTy.equals(argTy)) {
            var e = parTy.inspect(), g = argTy.inspect();
            throw new TypeError("Argument type mismatch at parameter " + (i + 1) + ", expected: " + e + ", got: " + g);
        }
    }
    node.type = new types.Instance(functionType.ret);
};
TypeSystem.prototype.visitChain = function (node, scope) {
    var self = this, headType = know(node, scope.get(node.name));
    // Save the type of the head
    node.headType = headType;
    // Start at the head of the chain
    var type = headType;
    for (var i = 0; i < node.tail.length; i++) {
        var item = node.tail[i];
        if (item instanceof AST.Call) {
            // Make sure we're trying to call an instance
            assertInstanceOf(type, types.Instance, 'Unexpected non-Instanced Function');
            // Get the type of the instance
            type = type.type;
            assertInstanceOf(type, types.Function, 'Trying to call non-Function');
            var typeArgs = type.args, itemArgs = item.args;
            // Check to make sure we're getting as many arguments as we expected
            if (typeArgs.length !== itemArgs.length) {
                var typeCount = typeArgs.length, itemCount = itemArgs.length;
                throw new TypeError('Wrong number of arguments: expected ' + typeCount + ', got ' + itemCount);
            }
            // Then type-check each individual arguments
            for (var argIdx = itemArgs.length - 1; argIdx >= 0; argIdx--) {
                // Visit each argument item
                var itemArg = itemArgs[argIdx];
                self.visitExpression(itemArg, scope);
                // Get the Instance type of the passing argument node
                var itemArgInstance = itemArg.type;
                // Verify that the passed argument's type is an Instance box
                var failureMessage = 'Expected Instance as function argument, got: ' + itemArgInstance.inspect();
                assertInstanceOf(itemArgInstance, types.Instance, failureMessage);
                // Unbox the instance
                var itemArgType = itemArgInstance.type;
                // Then get the type from the function definition to compare to the
                // passed argument
                var typeArg = typeArgs[argIdx];
                if (!typeArg.equals(itemArgType)) {
                    var message = 'Argument mismatch at argument index ' + i, got = itemArgType.inspect(), expected = typeArg.inspect();
                    message += "\n  expected " + expected + ', got ' + got;
                    throw new TypeError(message, item);
                }
            }
            // Replace current type with an instance of type that's going to be returned
            var returnType = type.ret;
            type = new types.Instance(returnType);
        }
        else if (item instanceof AST.Property) {
            type = this.getTypeOfTypesProperty(type, item.name);
        }
        else {
            throw new TypeError('Cannot handle Chain item of type: ' + item.constructor.name, node);
        }
    }
    node.type = type;
};
// Utility function for resolving the type of a type's property. Handles
// either Modules or Instances of a type; for everything else it will
// throw an error.
TypeSystem.prototype.getTypeOfTypesProperty = function (type, name) {
    var returnType = null;
    if (type instanceof types.Module) {
    }
    else {
        var typeName = (type ? type.inspect() : String(type));
        assertInstanceOf(type, types.Instance, 'Trying to get property of non-Instance: ' + typeName);
        var instance = type;
        // Unbox the instance
        type = instance.type;
    }
    returnType = type.getTypeOfProperty(name);
    // If it's another Module then just return that
    if (returnType instanceof types.Module) {
        return returnType;
    }
    // Otherwise box it into an instance
    return new types.Instance(returnType);
};
TypeSystem.prototype.visitMulti = function (node, scope) {
    var self = this;
    // Construct a new array of name-type args
    var args = node.args.map(function (arg) {
        var name = arg.name, type = self.resolveType(arg.type);
        return { name: name, type: type };
    });
    if (!node.ret) {
        throw new TypeError('Missing multi return type', node);
    }
    var ret = this.resolveType(node.ret);
    // Construct Multi type with the arguments and return types
    var multi = new types.Multi(this.rootObject, args, ret);
    node.type = multi;
    // Add multi to the scope
    scope.setLocal(node.name, multi);
};
module.exports = { TypeSystem: TypeSystem };
