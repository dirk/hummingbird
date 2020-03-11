# Hummingbird

[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/dirk/hummingbird.svg?branch=master
[travis-url]: https://travis-ci.org/dirk/hummingbird

This is an experimental language.

For the previous JavaScript implementation which compiled to JavaScript or binary-via-LLVM check out the [`legacy-2015`](https://github.com/dirk/hummingbird/tree/legacy-2015) branch.

## Getting started on macOS

One-time setup:

```sh
# Install LLVM v8
brew install llvm@8
```

When starting work in a new shell:

```sh
# Set up your environment so that the llvm-sys crate can discover Homebrew's
# installation of llvm@8.
source script/env-mac
```

Building and testing:

```sh
cargo build
cargo test
```

## License

Released under the Modified BSD License. See [LICENSE](LICENSE) for details.
