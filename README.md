# Hummingbird

For now refer to the [manual](doc/Manual.md).

## Specification

The Hummingbird [specification](doc/Specification.md) is designed to be both human- and
machine-readable. It is organized into sections for each syntactical and
semantic feature of the language.

Each feature has a `<spec name="..."></spec>` block containing the Hummingbird
example source and the expected JavaScript output. These can then be parsed and
a full suite of unit tests generated in `test/spec/`.

```bash
# Generating the spec tests
npm run gen-spec
# Running those tests
npm run test-spec
```

