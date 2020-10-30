# Changelog

## [Unreleased]


## [0.9.0] - 2020-10-30

- Added `magic` attribute, this allows the ability to specify a set of bytes
which must be present at the start of the data
([@samuelsleight](https://github.com/samuelsleight))
- Added `until` attribute, this allows the ability to read until a given predicate
([@samuelsleight](https://github.com/samuelsleight))
- Added `bits_read` and `bytes_read` container attributes, this allows the ability to specify
an amount of bits/bytes to read inside a Vec<T>
([@samuelsleight](https://github.com/samuelsleight))
- Improved documentation
- Refactored test cases
- Code improvements ([@wcampbell0x2a](https://github.com/wcampbell0x2a))

## [0.8.0] - 2020-09-29

- `write` now takes a `&mut BitVec` instead of returning a BitVec, this optimization
speeds up serialization ([@agausmann](https://github.com/agausmann))

The following items have been renamed: ([@wcampbell0x2a](https://github.com/wcampbell0x2a))
- Renamed `id_type` in favor of `type`
- Renamed `id_bits` in favor of `bits`
- Renamed `id_bytes` in favor of `bytes`

Internal:
- Updated criterion to latest
- Using tarpaulin for code coverage now
- Swapped hex! macro ([@inspier](https://github.com/inspier))
- Code improvements ([@wcampbell0x2a](https://github.com/wcampbell0x2a))

## [0.7.2] - 2020-09-02

- Added `ctx_default` attribute, this allows the ability to specify defaults to
types accepting a `ctx` if none are provided
- Updated documentation regarding the concept of context and how it
applies to some attributes
- Added validation to `id` attribute
- `endian` attribute now accepts an expression (still accepts `big` or `little`)
- Updated bitvec dependency

## [0.7.1] - 2020-07-31

- Added `id` attribute to top-level enums which allows to specify the enum id,
for example a value coming via a context variable

## [0.7.0] - 2020-07-28

- Added `cond` attribute which allows for conditional parsing or skipping of a field
- Added `id_pat` attribute which allows pattern matching for enum variants

Community:

- Added `ctx` attribute which adds the ability to pass context to child parsers from the parent ([@constfold](https://github.com/constfold))
- Internal refactoring of `endian`, `bits` and `count` attributes, they are now sugar around the `ctx` ([@constfold](https://github.com/constfold))
- Renamed `to_bitvec` to `to_bits` ([@wcampbell0x2a](https://github.com/wcampbell0x2a))

## [0.6.1] - 2020-07-06

- Enum variant specified without an `id` attribute is now considered the catch-all

## [0.6.0] - 2020-06-22

- Added `DekuContainerRead` and `DekuContainerWrite` to expose `from_bytes`, `to_bytes` and `to_bitvec`
- Added `release.toml`
- Added `CHANGELOG.md` to track changes

[Unreleased]: https://github.com/sharksforarms/deku/compare/deku-v0.9.0...HEAD

[0.9.0]: https://github.com/sharksforarms/deku/compare/deku-v0.8.0...deku-v0.9.0

[0.8.0]: https://github.com/sharksforarms/deku/compare/deku-v0.7.2...deku-v0.8.0

[0.7.2]: https://github.com/sharksforarms/deku/compare/deku-v0.7.1...deku-v0.7.2

[0.7.1]: https://github.com/sharksforarms/deku/compare/deku-v0.7.0...deku-v0.7.1

[0.7.0]: https://github.com/sharksforarms/deku/compare/deku-v0.6.1...deku-v0.7.0

[0.6.1]: https://github.com/sharksforarms/deku/compare/deku-v0.6.0...deku-v0.6.1
