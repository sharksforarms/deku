# Changelog

## [Unreleased]

## [0.11.0] - 2020-02-25

### Breaking
- Removed `bitvec` from `deku::prelude` ([#181](https://github.com/sharksforarms/deku/pull/181))
    - This will break custom `reader` and `writer` function definitions
    - `bitvec` is re-exported via: `deku::bitvec::` (this contains `bitvec::prelude::*`)

### Added
- Added `DekuEnumExt` to provide extra utility functions to enums. ([#176](https://github.com/sharksforarms/deku/pull/176))
    - This trait is implemented on enums derived with `#[derive(DekuRead)]`
    - This trait currently contains 1 function: `deku_id()`
    - `deku_id` can be called on an enum variant to get the deku `id` of the variant
- Added `Incomplete(NeedSize)` variant on `DekuError` ([#177](https://github.com/sharksforarms/deku/pull/177))
- Added `CODE_OF_CONDUCT.md`
- Code improvements ([@wcampbell0x2a](https://github.com/wcampbell0x2a))

## [0.10.1] - 2020-02-25

- Update bitvec dependency to fix [build failures](https://github.com/bitvecto-rs/bitvec/issues/105)

## [0.10.0] - 2020-01-09
### Breaking
- Enum's which don't specify an `id` attribute now default to their discriminant value
instead of being treated as a catch-all ([#139](https://github.com/sharksforarms/deku/pull/139))
- Removed `BitSize` in favor of a new enum `Size` with two variants, `Bits` and `Bytes` ([#138](https://github.com/sharksforarms/deku/pull/138))
- Added namespacing to internal variables. `deku::` is used to access internal variables in token fields. ([#150](https://github.com/sharksforarms/deku/pull/150))
For example, `reader = "my_reader(deku::rest, deku::bit_offset)"` or `writer = "my_writer(deku::output)"`
- Introduced a lifetime to `DekuRead` in support of zero-copy reading ([#158](https://github.com/sharksforarms/deku/pull/158))

### Added
- Zero-copy reading on &[u8] ([#158](https://github.com/sharksforarms/deku/pull/158))
- Padding related attributes: `pad_bits_before`, `pad_bytes_before`, `pad_bits_after`, `pad_bytes_after` ([#163](https://github.com/sharksforarms/deku/pull/163))
- Assertion related attributes: `assert`, `assert_eq` ([#164](https://github.com/sharksforarms/deku/pull/164))
- Ability to use more types in enum's `type` attribute ([#162](https://github.com/sharksforarms/deku/pull/162))
- Ability to use LitByteStr in enum's `id` attribute, for example `id = b"0x01"` ([#162](https://github.com/sharksforarms/deku/pull/162))
- Access to read offset via bit_offset and byte_offset internal variables. ([#149](https://github.com/sharksforarms/deku/pull/149))
These are accessed via `deku::` namespace, `deku::bit_offset` and `deku::byte_offset`. ([#150](https://github.com/sharksforarms/deku/pull/150))
- `#[deku(temp)]` attribute, enabled via `deku_derive` proc-macro attribute. ([#136](https://github.com/sharksforarms/deku/pull/136))
This allows reading/use of a field without it being stored in the container.
- DekuRead+DekuWrite implementations for CString ([#144](https://github.com/sharksforarms/deku/pull/144))
- DekuRead+DekuWrite implementations for NonZeroT types ([#140](https://github.com/sharksforarms/deku/pull/140))
- DekuRead+DekuWrite implementations for bool ([#161](https://github.com/sharksforarms/deku/pull/161))
- DekuRead+DekuWrite implementations for () ([#159](https://github.com/sharksforarms/deku/pull/159))
- DekuRead+DekuWrite implementations for Box<T> and Box<[T]> ([#160](https://github.com/sharksforarms/deku/pull/160))

### Other
- Internal code/test refactoring
- Code improvements ([@myrrlyn](https://github.com/myrrlyn), [@wcampbell0x2a](https://github.com/wcampbell0x2a), [@inspier](https://github.com/inspier))

## [0.9.3] - 2020-12-14
- Patch release to fix semver break in darling,
[this has since been fixed](https://github.com/TedDriggs/darling/issues/107)

## [0.9.2 - yanked] - 2020-12-14
- Patch release to fix semver break in darling,
[this has since been fixed](https://github.com/TedDriggs/darling/issues/107)

## [0.9.1] - 2020-10-31
- Changed minimum bitvec version to 0.19.4 to have desired `offset_from`
functionality (https://github.com/myrrlyn/bitvec/issues/86). This was missed in
0.9.0 release.
- Code improvements ([@wcampbell0x2a](https://github.com/wcampbell0x2a))

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

[Unreleased]: https://github.com/sharksforarms/deku/compare/deku-v0.11.0...HEAD

[0.11.0]: https://github.com/sharksforarms/deku/compare/deku-v0.10.1...deku-v0.11.0

[0.10.1]: https://github.com/sharksforarms/deku/compare/deku-v0.10.0...deku-v0.10.1

[0.10.0]: https://github.com/sharksforarms/deku/compare/deku-v0.9.3...deku-v0.10.0

[0.9.3]: https://github.com/sharksforarms/deku/compare/deku-v0.9.1...deku-v0.9.3

[0.9.2]: https://github.com/sharksforarms/deku/compare/deku-v0.9.1...deku-v0.9.2

[0.9.1]: https://github.com/sharksforarms/deku/compare/deku-v0.9.0...deku-v0.9.1

[0.9.0]: https://github.com/sharksforarms/deku/compare/deku-v0.8.0...deku-v0.9.0

[0.8.0]: https://github.com/sharksforarms/deku/compare/deku-v0.7.2...deku-v0.8.0

[0.7.2]: https://github.com/sharksforarms/deku/compare/deku-v0.7.1...deku-v0.7.2

[0.7.1]: https://github.com/sharksforarms/deku/compare/deku-v0.7.0...deku-v0.7.1

[0.7.0]: https://github.com/sharksforarms/deku/compare/deku-v0.6.1...deku-v0.7.0

[0.6.1]: https://github.com/sharksforarms/deku/compare/deku-v0.6.0...deku-v0.6.1
