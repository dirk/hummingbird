# LLVM target compiler

This is the unstable backend targeting native compilation through the LLVM and native toolchains. Notes and other information about this compiler are below.

### Name mangling

Names are mangled by prepending a tag (based on the type of the name's value); names in a path (modules, type classes, etc.) are separated by underscores.

##### Compilation tags

- Interfaces: (non-value)
  - **M**: module
  - **T**: type (class)
  - **m**: method
- Values:
  - **A**: anonymous function value
  - **F**: named function value
  - **G**: global value (constant)

##### Examples of some compilation tags:

For the `concat` function in `std.core.types.string`:

```
Mstd_mcore_mtypes_mstring_fconcat
```

For the `log` method of an instance of the `BuiltinConsole` class type:

```
TBuiltinConsole_mlog
```

