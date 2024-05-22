# Changelog

## [Unreleased]

## Changes
- Bumped MSRV to `1.71` ([#438](https://github.com/sharksforarms/deku/pull/438))
- Add DekuWrite impl for `[T]` ([#416](https://github.com/sharksforarms/deku/pull/416))
- Add `no-assert-string` feature to remove panic string on failed assertion ([#405](https://github.com/sharksforarms/deku/pull/405))
- Add `read_all` attribute to read until `reader.end()` ([#387](https://github.com/sharksforarms/deku/pull/387))
- Changed edition to 2021 ([#389](https://github.com/sharksforarms/deku/pull/389))
- Refactored `logging` feature with massive usability increases ([#352](https://github.com/sharksforarms/deku/pull/352)), ([#355](https://github.com/sharksforarms/deku/pull/355))
- Bumped the `syn` library to 2.0, which required replacing `type` for Enums with `id_type` ([#386](https://github.com/sharksforarms/deku/pull/386))
```diff,rust
 #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
-#[deku(type = "u8")]
+#[deku(id_type = "u8")]
 enum DekuTest {
     #[deku(id_pat = "_")]
     VariantC((u8, u8)),
 }
```

### Updated Reader API
- Changed API of reading to use `io::Read`, bringing massive performance and usability improvements ([#352](https://github.com/sharksforarms/deku/pull/352))
- Changed the trait `DekuRead` to `DekuReader`

For example:
```rust
use std::io::{Seek, SeekFrom, Read};
use std::fs::File;
use deku::prelude::*;

#[derive(Debug, DekuRead, DekuWrite, PartialEq, Eq, Clone, Hash)]
#[deku(endian = "big")]
struct EcHdr {
    magic: [u8; 4],
    version: u8,
    padding1: [u8; 3],
}

let mut file = File::options().read(true).open("file").unwrap();
let ec = EcHdr::from_reader((&mut file, 0)).unwrap();
```

- The more internal (with context) `read(..)` was replaced with `from_reader_with_ctx(..)`.
With the switch to internal streaming, the variables `deku::input`, `deku::input_bits`, and `deku::rest` are now not possible and were removed.
`deku::reader` is a replacement for some of the functionality.
See [examples/deku_input.rs](examples/deku_input.rs) for a new example of caching all reads.

Old:
```rust
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,

    #[deku(
        reader = "bit_flipper_read(*field_a, deku::rest, BitSize(8))",
    )]
    field_b: u8,
}

fn custom_read(
    field_a: u8,
    rest: &BitSlice<u8, Msb0>,
    bit_size: BitSize,
) -> Result<(&BitSlice<u8, Msb0>, u8), DekuError> {

    // read field_b, calling original func
    let (rest, value) = u8::read(rest, bit_size)?;

    Ok((rest, value))
}
```

New:
```rust
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,

    #[deku(
        reader = "bit_flipper_read(*field_a, deku::reader, BitSize(8))",
    )]
    field_b: u8,
}

fn custom_read<R: std::io::Read>(
    field_a: u8,
    reader: &mut Reader<R>,
    bit_size: BitSize,
) -> Result<u8, DekuError> {

    // read field_b, calling original func
    let value = u8::from_reader_with_ctx(reader, bit_size)?;

    Ok(value)
}
```

- With the addition of using `Read`, containing a byte slice with a reference is not supported:

Old
```rust
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct TestStruct<'a> {
    bytes: u8,

    #[deku(bytes_read = "bytes")]
    data: &'a [u8],
}
```

New
```rust
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct TestStruct {
    bytes: u8,

    #[deku(bytes_read = "bytes")]
    data: Vec<u8>,
}
```

- `id_pat` is now required to be the same type as stored id.
This also disallows using tuples for storing the id:

Old:
```rust
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum DekuTest {
    #[deku(id_pat = "_")]
    VariantC((u8, u8)),
}
```

New:
```rust
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum DekuTest {
    #[deku(id_pat = "_")]
    VariantC {
        id: u8,
        other: u8,
    },
}
```

- The feature `const_generics` was removed and is enabled by default.

### Updated Writer API
- Changed API of writing to use `io::Write`, bringing massive performance and usability improvements ([#355](https://github.com/sharksforarms/deku/pull/355))
- Changed the trait `DekuWrite` to `DekuWriter`
- The more internal (with context) `write(..)` was replaced with `to_writer(..)`.
With the switch to internal streaming, the variables `deku::output` are now not possible and were removed. `deku::writer` is a replacement for some of the functionality.

Old:
```rust
fn bit_flipper_write(
    field_a: u8,
    field_b: u8,
    output: &mut BitVec<u8, Msb0>,
    bit_size: BitSize,
) -> Result<(), DekuError> {
    // Access to previously written fields
    println!("field_a = 0x{:X}", field_a);

    // value of field_b
    println!("field_b = 0x{:X}", field_b);

    // Size of the current field
    println!("bit_size: {:?}", bit_size);

    // flip the bits on value if field_a is 0x01
    let value = if field_a == 0x01 { !field_b } else { field_b };

    value.write(output, bit_size)
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,

    #[deku(
        writer = "bit_flipper_write(*field_a, *field_b, deku::output, BitSize(8))"
    )]
    field_b: u8,
}
````

New:
```rust
fn bit_flipper_write<W: Write>(
    field_a: u8,
    field_b: u8,
    writer: &mut Writer<W>,
    bit_size: BitSize,
) -> Result<(), DekuError> {
    // Access to previously written fields
    println!("field_a = 0x{:X}", field_a);

    // value of field_b
    println!("field_b = 0x{:X}", field_b);

    // Size of the current field
    println!("bit_size: {:?}", bit_size);

    // flip the bits on value if field_a is 0x01
    let value = if field_a == 0x01 { !field_b } else { field_b };

    value.to_writer(writer, bit_size)
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,

    #[deku(
        writer = "bit_flipper_write(*field_a, *field_b, deku::writer, BitSize(8))"
    )]
    field_b: u8,
}
```
- Added `DekuError::Write` to denote `io::Write` errors

## Bug fix
- Fix error for invalid deku_id generation on generic enum ([#411](https://github.com/sharksforarms/deku/pull/411))

## [0.16.0] - 2023-02-28

### Changes
- Faster build times: Optimize derive macros ([@dullbananas](https://github.com/dullbananas)) ([#320](https://github.com/sharksforarms/deku/pull/320))
- Support for multiple arguments in enum `id` ([@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#315](https://github.com/sharksforarms/deku/pull/315))

### Bug fix
- Fixes [#264](https://github.com/sharksforarms/deku/issues/264) reported by [wildbook](https://github.com/wildbook): Drop MaybeUninit when failing to read entire slice ([@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#317](https://github.com/sharksforarms/deku/pull/317))

## [0.15.1] - 2022-12-19

Small bug fix: Use fully qualified path when calling `write` as it may clash with other impls

## [0.15.0] - 2022-11-16

### Breaking/Performance

- Upgrade to bitvec 1.0.0 may cause some breaking changes in some code bases.
- Performance note: Upgrade to bitvec 1.0.0 has shown negative performance impacts.

### Changes

- Upgrade to bitvec 1.0.0 ([@JuanPotato](https://github.com/JuanPotato) & [@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#281](https://github.com/sharksforarms/deku/pull/281))
- impl From<DekuError> for std::io::Error ([@caass](https://github.com/caass)) ([#285](https://github.com/sharksforarms/deku/pull/285))
- Fix typo in docs ([@vidhanio](https://github.com/vidhanio)) ([#291](https://github.com/sharksforarms/deku/pull/291))

### Bug fix

- Fix regression with unaligned u8 ([@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#294](https://github.com/sharksforarms/deku/pull/294))

## [0.14.1] - 2022-10-09

### Bug fix

- Fix issue where endianness trickery wouldn't swap bytes correctly when reading less than the full amount into an integer ([@caass](https://github.com/caass)) ([#283](https://github.com/sharksforarms/deku/pull/283))

### Changed

- Constify primitive bit size of types ([@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#279](https://github.com/sharksforarms/deku/pull/279))

## [0.14.0] - 2022-10-06

This release introduces a performance specialization/optimization in the read path for bytes

### Breaking
- `Size` enum removed in favor of `BitSize` and `ByteSize`, this change is to allow a performance optimization in the byte reading
- Byte specialization ([@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#278](https://github.com/sharksforarms/deku/pull/278))

### Added
- Add logging feature ([@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#271](https://github.com/sharksforarms/deku/pull/271))

## [0.13.1] - 2022-06-09

-  Documentation fix for `Size::byte_size` ([@korrat](https://github.com/korrat)) ([#261](https://github.com/sharksforarms/deku/pull/261))
-  Derive `Clone` on `DekuError` ([@interruptinuse](https://github.com/interruptinuse)) ([#255](https://github.com/sharksforarms/deku/pull/255))

# 🚨 NOTICE 🚨

- Fixed undefined behavior in the use of `MaybeUninit` in slice implementation ([#254](https://github.com/sharksforarms/deku/pull/254))
- Backported fix to 0.12 series as 0.12.6 and yanked affected versions (0.13.0, 0.12.5, 0.12.4)

## [0.13.0] - 2022-02-26

- Fixed no_std example/tests ([@korrat](https://github.com/korrat)) ([#247](https://github.com/sharksforarms/deku/pull/247))

### Breaking
- Sign extend integers ([@korrat](https://github.com/korrat)) ([#238](https://github.com/sharksforarms/deku/pull/238))

## [0.12.6] - 2022-06-09
- (Backport) Fixed undefined behavior in the use of `MaybeUninit` in slice implementation ([#254](https://github.com/sharksforarms/deku/pull/254))

## [0.12.5] - 2021-11-11
- Show struct ident in assertion error message ([@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#239](https://github.com/sharksforarms/deku/pull/239))

## [0.12.4] - 2021-10-11
- Add Read/Write impls for generic arrays ([@xlambein](https://github.com/xlambein)) ([#235](https://github.com/sharksforarms/deku/pull/235))

## [0.12.3] - 2021-07-06
- Bug fix for structs/enums which also define a `to_bytes` function

## [0.12.2] - 2021-06-15
- Bug fix for in primitive bitslice convertion ([issue](https://github.com/sharksforarms/deku/issues/224)/[commit](https://github.com/sharksforarms/deku/commit/8d58bc6c65a9f3305d22ebe1bb5a685c39295863))
- Added Miri tests including testing on big endian target ([#211](https://github.com/sharksforarms/deku/pull/211))

## [0.12.1] - 2021-05-25
- Add support for raw indentifiers ([@Visse](https://github.com/visse)) ([#221](https://github.com/sharksforarms/deku/pull/221))
- Dependency updates

## [0.12.0] - 2021-04-13

### Breaking
- `const_generics` feature enabled by default

### Added
- Add optional `const_generics` feature ([@soruh](https://github.com/soruh)) ([#187](https://github.com/sharksforarms/deku/pull/187))
    - This allows for reading/writing arrays with >32 elements when enabled
- DekuRead+DekuWrite implementations for Cow<T> ([@abungay](https://github.com/abungay)) ([#186](https://github.com/sharksforarms/deku/pull/186))
- DekuRead+DekuWrite implementations for HashMap ([@abungay](https://github.com/abungay)) ([#199](https://github.com/sharksforarms/deku/pull/199))
- DekuRead+DekuWrite implementations for HashSet ([@abungay](https://github.com/abungay)) ([#199](https://github.com/sharksforarms/deku/pull/199))
- DekuWrite implementations for &T ([@abungay](https://github.com/abungay)) ([#199](https://github.com/sharksforarms/deku/pull/199))
- DekuRead+DekuWrite implementations for tuple ([@abungay](https://github.com/abungay)) ([#198](https://github.com/sharksforarms/deku/pull/198))

### Other
- Updated dependencies
- Updated wasm example
- Fix: Parenthesize pad/update attributes ([@wcampbell0x2a](https://github.com/wcampbell0x2a)) ([#195](https://github.com/sharksforarms/deku/pull/195))
- Fixed failing code coverage ([@abungay](https://github.com/abungay)) ([#200](https://github.com/sharksforarms/deku/pull/200))
- Update DekuRead documentation ([@caass](https://github.com/caass)) ([fcfdc24](https://github.com/sharksforarms/deku/commit/fcfdc24eca1b8663f9a2cd2d0d8ad6534b08a862)/[#196](https://github.com/sharksforarms/deku/pull/196))
- Updated hexlit dependency ([@inspier](https://github.com/inspier)) ([#189](https://github.com/sharksforarms/deku/pull/189))
- Refactoring and code improvements ([@wcampbell0x2a](https://github.com/wcampbell0x2a))

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

[Unreleased]: https://github.com/sharksforarms/deku/compare/deku-v0.16.0...HEAD

[0.16.0]: https://github.com/sharksforarms/deku/compare/deku-v0.15.1...deku-v0.16.0

[0.15.1]: https://github.com/sharksforarms/deku/compare/deku-v0.15.0...deku-v0.15.1

[0.15.0]: https://github.com/sharksforarms/deku/compare/deku-v0.14.1...deku-v0.15.0

[0.14.1]: https://github.com/sharksforarms/deku/compare/deku-v0.14.0...deku-v0.14.1

[0.14.0]: https://github.com/sharksforarms/deku/compare/deku-v0.13.1...deku-v0.14.0

[0.13.1]: https://github.com/sharksforarms/deku/compare/deku-v0.13.0...deku-v0.13.1

[0.13.0]: https://github.com/sharksforarms/deku/compare/deku-v0.12.6...deku-v0.13.0

[0.12.6]: https://github.com/sharksforarms/deku/compare/deku-v0.12.5...deku-v0.12.6

[0.12.5]: https://github.com/sharksforarms/deku/compare/deku-v0.12.4...deku-v0.12.5

[0.12.4]: https://github.com/sharksforarms/deku/compare/deku-v0.12.3...deku-v0.12.4

[0.12.3]: https://github.com/sharksforarms/deku/compare/deku-v0.12.2...deku-v0.12.3

[0.12.2]: https://github.com/sharksforarms/deku/compare/deku-v0.12.1...deku-v0.12.2

[0.12.1]: https://github.com/sharksforarms/deku/compare/deku-v0.12.0...deku-v0.12.1

[0.12.0]: https://github.com/sharksforarms/deku/compare/deku-v0.11.0...deku-v0.12.0

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
