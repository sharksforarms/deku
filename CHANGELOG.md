# Changelog

## [0.7.0 - Unreleased]

- Added `cond` attribute, which allows for conditional parsing or skipping of a field

Community:

- Added `ctx` attribute which adds the ability to pass context to child parsers from the parent ([@constfold](https://github.com/constfold))
- Internal refactoring of `endian`, `bits` and `count` attributes, they are now sugar around the `ctx` ([@constfold](https://github.com/constfold))
- Refactored `test_macros.rs` ([@wcampbell0x2a](https://github.com/wcampbell0x2a))


## [0.6.1] - 2020-07-06

- Enum variant specified without an `id` attribute is now considered the catch-all

## [0.6.0] - 2020-06-22

- Added `DekuContainerRead` and `DekuContainerWrite` to expose `from_bytes`, `to_bytes` and `to_bitvec`
- Added `release.toml`
- Added `CHANGELOG.md` to track changes

[Unreleased]: https://github.com/sharksforarms/deku/compare/deku-v0.6.1...HEAD

[0.6.1]: https://github.com/sharksforarms/deku/compare/deku-v0.6.0...deku-v0.6.1
