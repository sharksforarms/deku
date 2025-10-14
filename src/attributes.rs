/*!
A documentation-only module for #\[deku\] attributes

# Scopes

To understand the `Scope` column of the table below:

```rust,ignore
#[deku(/* top-level */)]
struct DekuStruct {
    #[deku( /* field */)]
    field: u8,
}

#[deku(/* top-level */)]
enum DekuEnum {
    #[deku(/* variant */)]
    VariantA,

    #[deku(/* variant */)]
    VariantB( #[deku(/* field */)] u8 ),

    #[deku(/* variant */)]
    VariantC {
        #[deku( /* field */)]
        field: u8,
    }
}
```

# List of attributes

| Attribute | Scope | Description
|-----------|------------------|------------
| [endian](#endian) | top-level, field | Set the endianness
| [bit_order](#bit_order) | top-level, field | Set the bit-order when reading bits
| [magic](#magic) | top-level, field | A magic value that must be present at the start of this struct/enum/field
| [seek_from_current](#seek_from_current) | top-level, field | Sets the offset of reader and writer to the current position plus the specified number of bytes
| [seek_from_end](#seek_from_end) | top-level, field | Sets the offset to the size of reader and writer plus the specified number of bytes
| [seek_from_start](#seek_from_start) | top-level, field | Sets the offset of reader and writer to provided number of bytes
| [seek_rewind](#seek_rewind) | top-level, field | Rewind the reader and writer to the beginning
| [assert](#assert) | field | Assert a condition
| [assert_eq](#assert_eq) | field | Assert equals on the field
| [bits](#bits) | field | Set the bit-size of the field
| [bytes](#bytes) | field | Set the byte-size of the field
| [count](#count) | field | Set the field representing the element count of a container
| [bits_read](#bits_read) | field | Set the field representing the number of bits to read into a container
| [bytes_read](#bytes_read) | field | Set the field representing the number of bytes to read into a container
| [until](#until) | field | Set a predicate returning when to stop reading elements into a container
| [read_all](#read_all) | field | Read until [reader.end()] returns `true`
| [update](#update) | field | Apply code over the field when `.update()` is called
| [temp](#temp) | field | Read the field but exclude it from the struct/enum
| [temp_value](#temp_value) | field | Write the field but exclude it from the struct/enum
| [skip](#skip) | field | Skip the reading/writing of a field
| [pad_bytes_before](#pad_bytes_before) | field | Skip bytes before reading, pad before writing
| [pad_bits_before](#pad_bits_before) | field | Skip bits before reading, pad before writing
| [pad_bytes_after](#pad_bytes_after) | field | Skip bytes after reading, pad after writing
| [pad_bits_after](#pad_bits_after) | field | Skip bits after reading, pad after writing
| [cond](#cond) | field | Conditional expression for the field
| [default](#default) | field | Provide default value. Used with [skip](#skip) or [cond](#cond)
| [map](#map) | field | Specify a function or lambda to apply to the result of the read
| [reader](#readerwriter) | variant, field | Custom reader code
| [writer](#readerwriter) | variant, field | Custom writer code
| [ctx](#ctx) | top-level, field| Context list for context sensitive parsing
| [ctx_default](#ctx_default) | top-level, field| Default context values
| enum: [id](#id) | top-level, variant | enum or variant id value
| enum: [id_endian](#id_endian) | top-level | Endianness of *just* the enum `id`
| enum: [id_pat](#id_pat) | variant | variant id match pattern
| enum: [id_type](#id_type) | top-level | Set the type of the variant `id`
| enum: [bits](#bits-1) | top-level | Set the bit-size of the variant `id`
| enum: [bytes](#bytes-1) | top-level | Set the byte-size of the variant `id`

# endian

Set to read/write bytes in a specific byte order.

Values: `big`, `little` or an expression which returns a [`Endian`](super::ctx::Endian)

Precedence: field > top-level > system endianness (default)

Example:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")] // top-level, defaults to system endianness
struct DekuTest {
    #[deku(endian = "big")] // field-level override
    field_be: u16,
    field_default: u16, // defaults to top-level
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0xAB, 0xCD, 0xAB, 0xCD];
let mut cursor = Cursor::new(data);

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       field_be: 0xABCD,
       field_default: 0xCDAB,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, &*value);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

**Note**: The `endian` is passed as a context argument to sub-types

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")] // context passed from `DekuTest` top-level endian
struct Child {
    field_a: u16
}

# #[cfg(feature = "alloc")]
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")] // top-level, defaults to system endianness
struct DekuTest {
    #[deku(endian = "big")] // field-level override
    field_be: u16,
    field_default: u16, // defaults to top-level

    // because a top-level endian is specified,
    // it is passed as a context
    field_child: Child,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0xAB, 0xCD, 0xAB, 0xCD, 0xEF, 0xBE];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       field_be: 0xABCD,
       field_default: 0xCDAB,
       field_child: Child { field_a: 0xBEEF }
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(&*data, value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# bit_order
Specify the field or containers bit order. By default all bits are read in `Msb0` (Most significant bit) order.
### Top-Level Example
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "bits")]
# #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(bit_order = "lsb")]
pub struct SquashfsV3 {
    #[deku(bits = "4")]
    inode_type: u8,
    #[deku(bits = "12")]
    mode: u16,
    uid: u8,
    guid: u8,
    mtime: u32,
    inode_number: u32,
}

# #[cfg(feature = "bits")]
# fn main() {
let data: &[u8] = &[
//       inode_type
//     ╭-----------
//     |
//     |    mode
//    ╭+--------   ...and so on
//    ||    ||
//    vv    vv
    0x31, 0x12, 0x04, 0x05, 0x06, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
];
let header = SquashfsV3::try_from(data).unwrap();
assert_eq!(
    SquashfsV3 {
        inode_type: 0x01,
        mode: 0x123,
        uid: 0x4,
        guid: 0x5,
        mtime: 0x6,
        inode_number: 0x7
    },
    header,
);
# }
#
# #[cfg(not(feature = "bits"))]
# fn main() {}
```
With endian-ness:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "bits")]
# #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(endian = "big", bit_order = "lsb")]
pub struct BigEndian {
    #[deku(bits = "13")]
    offset: u16,
    #[deku(bits = "3")]
    t: u8,
}

# #[cfg(all(feature = "alloc", feature = "bits"))]
# fn main() {
let data = vec![0x10, 0x81];
let big_endian = BigEndian::try_from(data.as_ref()).unwrap();
assert_eq!(
    big_endian,
    BigEndian {
        offset: 0x1001,
        t: 0b100,
    }
);
let bytes = big_endian.to_bytes().unwrap();
assert_eq!(bytes, data);
# }
#
# #[cfg(not(all(feature = "alloc", feature = "bits")))]
# fn main() {}
````
### Field Example
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "bits")]
# #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
pub struct LsbField {
    #[deku(bit_order = "lsb", bits = "13")]
    offset: u16,
    #[deku(bit_order = "lsb", bits = "3")]
    t: u8,
}

# #[cfg(all(feature = "alloc", feature = "bits"))]
# fn main() {
let data = vec![0x40, 0x40];
let more_first = LsbField::try_from(data.as_ref()).unwrap();
assert_eq!(more_first, LsbField { offset: 0x40, t: 2 });
let bytes = more_first.to_bytes().unwrap();
assert_eq!(bytes, data);
# }
#
# #[cfg(not(all(feature = "alloc", feature = "bits")))]
# fn main() {}
```

# magic

Sets a "magic" value that must be present in the data at the start of
a struct/enum or field when reading, and that is written out of the start of
that type's data when writing.

Example (top-level):
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(magic = b"deku")]
struct DekuTest {
    data: u8
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[b'd', b'e', b'k', b'u', 50];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { data: 50 },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

Example (field):
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(magic = b"deku")]
    data: u8
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[b'd', b'e', b'k', b'u', 50];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { data: 50 },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# seek_from_current

Using the internal reader, seek to current position plus offset before reading field.

Field Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    // how many following bytes to skip
    skip_u8: u8,
    #[deku(seek_from_current = "*skip_u8")]
    byte: u8,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0x00, 0x02];
let mut cursor = Cursor::new(data);

let (_amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest { skip_u8: 0x01, byte: 0x02 },
    value
);

let bytes = value.to_bytes().unwrap();
assert_eq!(bytes, data);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

Top-Level Example (with ctx usage):

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(seek_from_current = "skip", ctx = "skip: usize")]
struct DekuTest {
    byte: u8,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x00, 0x02];
let mut cursor = Cursor::new(data);
let mut reader = Reader::new(&mut cursor);

let value = DekuTest::from_reader_with_ctx(&mut reader, 1).unwrap();

assert_eq!(
    DekuTest { byte: 0x02 },
    value
);

let mut buf = vec![];
let mut cursor = Cursor::new(&mut buf);
let mut writer = Writer::new(&mut cursor);
let bytes = value.to_writer(&mut writer, 1).unwrap();
assert_eq!(buf, data);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

# seek_from_end

Using the internal reader, seek to size of reader plus offset before reading field.

Field Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(seek_from_end = "-2")]
    byte: u8,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0xff, 0x02];
let mut cursor = Cursor::new(data);

let (_amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest { byte: 0xff },
    value
);

// NOTE: to_bytes() doesn't work, because we need `seek_from_end` to already
// have a correct allocated buffer length!
let mut buf = vec![0x01, 0x00, 0x02];
let mut cursor = Cursor::new(&mut buf);
let mut writer = Writer::new(&mut cursor);
let _ = value.to_writer(&mut writer, ()).unwrap();
assert_eq!(buf, data);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

Top-Level Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(seek_from_end = "-2")]
struct DekuTest {
    byte: u8,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0xff, 0x02];
let mut cursor = Cursor::new(data);

let (_amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest { byte: 0xff },
    value
);

// NOTE: to_bytes() doesn't work, because we need `seek_from_end` to already
// have a correct allocated buffer length!
let mut buf = vec![0x01, 0x00, 0x02];
let mut cursor = Cursor::new(&mut buf);
let mut writer = Writer::new(&mut cursor);
let _ = value.to_writer(&mut writer, ()).unwrap();
assert_eq!(buf, data);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

# seek_from_start

Using the internal reader, seek from reader start plus offset before reading field.

Field Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(seek_from_start = "2")]
    byte: u8,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0xff, 0x02];
let mut cursor = Cursor::new(data);

let (_amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest { byte: 0x02 },
    value
);

// NOTE: to_bytes() doesn't work, because we need `seek_from_start` to already
// have a correct allocated buffer length!
let mut buf = vec![0x01, 0xff, 0x00];
let mut cursor = Cursor::new(&mut buf);
let mut writer = Writer::new(&mut cursor);
let _ = value.to_writer(&mut writer, ()).unwrap();
assert_eq!(buf, data);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

Top-Level Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(seek_from_start = "2")]
struct DekuTest {
    byte: u8,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0xff, 0x02];
let mut cursor = Cursor::new(data);

let (_amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest { byte: 0x02 },
    value
);

// NOTE: to_bytes() doesn't work, because we need `seek_from_start` to already
// have a correct allocated buffer length!
let mut buf = vec![0x01, 0xff, 0x00];
let mut cursor = Cursor::new(&mut buf);
let mut writer = Writer::new(&mut cursor);
let _ = value.to_writer(&mut writer, ()).unwrap();
assert_eq!(buf, data);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

# seek_rewind

Rewind the internal reader to starting position.

Field Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
# #[cfg(feature = "std")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    byte_01: u8,
    #[deku(seek_rewind)]
    byte_02: u8,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0xff];
let mut cursor = Cursor::new(data);

let (_amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest { byte_01: 0xff, byte_02: 0xff },
    value
);
let bytes = value.to_bytes().unwrap();
assert_eq!(bytes, data);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

Top-Level Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
# #[cfg(feature = "std")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(seek_rewind)]
struct DekuTest {
    byte: u8,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0xff];
let mut cursor = Cursor::new(data);

let (_amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest { byte: 0xff},
    value
);
let bytes = value.to_bytes().unwrap();
assert_eq!(bytes, data);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```


# assert

Assert a condition after reading and before writing a field

Example:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(assert = "*data >= 8")]
    data: u8
}

let data: &[u8] = &[0x00, 0x01, 0x02];

let value = DekuTest::try_from(data);

#[cfg(feature = "descriptive-errors")]
assert_eq!(
    Err(DekuError::Assertion("Field failed assertion: DekuTest.data: * data >= 8".into())),
    value
);
#[cfg(not(feature = "descriptive-errors"))]
assert_eq!(
    Err(DekuError::Assertion("Field failed assertion".into())),
    value
);
```

# assert_eq

Assert equals after reading and before writing a field

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(assert_eq = "0x01")]
    data: u8,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0x01];

let mut value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { data: 0x01 },
    value
);

value.data = 0x02;

let value: Result<Vec<u8>, DekuError> = value.try_into();

# #[cfg(feature = "descriptive-errors")]
assert_eq!(
    Err(DekuError::Assertion("Field failed assertion: DekuTest.data: data == 0x01".into())),
    value
);
# #[cfg(not(feature = "descriptive-errors"))]
# assert_eq!(
#    Err(DekuError::Assertion("Field failed assertion".into())),
#    value
# );
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# bits

Set the bit-size of the field

**Note**: Cannot be used in combination with [bytes](#bytes)

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "bits")]
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bits = 2)]
    field_a: u8,
    #[deku(bits = 6)]
    field_b: u8,
    field_c: u8, // defaults to size_of<u8>*8
}

# #[cfg(all(feature = "alloc", feature = "bits"))]
# fn main() {
let data: &[u8] = &[0b11_101010, 0xFF];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       field_a: 0b11,
       field_b: 0b101010,
       field_c: 0xFF,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(&*data, value);
# }
#
# #[cfg(not(all(feature = "alloc", feature = "bits")))]
# fn main() {}
```

This attribute can also be set from a previous read:

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "bits")]
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    field_a_len: u8,
    #[deku(bits = "*field_a_len as usize")]
    field_a: u8,
    #[deku(bits = 6)]
    field_b: u8,
}

# #[cfg(all(feature = "alloc", feature = "bits"))]
# fn main() {
let data: &[u8] = &[0x02, 0b11_101010];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       field_a_len: 2,
       field_a: 0b11,
       field_b: 0b101010,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(&*data, value);
# }
#
# #[cfg(not(all(feature = "alloc", feature = "bits")))]
# fn main() {}
```


# bytes

Set the byte-size of the field

**Note**: Cannot be used in combination with [bits](#bits)

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bytes = 2)]
    field_a: u32,
    field_b: u8, // defaults to size_of<u8>
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0xAB, 0xCD, 0xFF];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       field_a: 0xCDAB,
       field_b: 0xFF,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

This attribute can also be set from a previous read:

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    field_a_size: u8,
    #[deku(bytes = "*field_a_size as usize")]
    field_a: u32,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0x03, 0x01, 0x02, 0x03];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       field_a_size: 0x03,
       field_a: 0x030201,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# count

Specify the field representing the length of the container, i.e. a Vec

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "alloc")]
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(update = "self.items.len()")]
    count: u8,
    #[deku(count = "count")]
    items: Vec<u8>,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0x02, 0xAB, 0xCD];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       count: 0x02,
       items: vec![0xAB, 0xCD],
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

**Note**: See [update](#update) for more information on the attribute!

## Specializations
- `Vec<u8>`: `count` used with a byte vector will result in one invocation to `read_bytes`, thus improving performance.

# bytes_read

Specify the field representing the total number of bytes to read into a container

See the following example, where `InnerDekuTest` is 2 bytes, so setting `bytes_read` to
4 will read 2 items into the container:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct InnerDekuTest {
    field_a: u8,
    field_b: u8
}

# #[cfg(feature = "alloc")]
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(update = "(self.items.len() / 2)")]
    bytes: u8,

    #[deku(bytes_read = "bytes")]
    items: Vec<InnerDekuTest>,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0x04, 0xAB, 0xBC, 0xDE, 0xEF];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       bytes: 0x04,
       items: vec![
           InnerDekuTest{field_a: 0xAB, field_b: 0xBC},
           InnerDekuTest{field_a: 0xDE, field_b: 0xEF}],
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(&*data, value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

**Note**: See [update](#update) for more information on the attribute!


# bits_read

This is equivalent to [bytes_read](#bytes_read), however specifies the bit limit instead
of a byte limit


# until

Specifies a predicate which sets when to stop reading values into the container.

**Note**: The last value which matches the predicate is read

The predicate is given a borrow to each item as it is read, and must return a boolean
as to whether this should be the last item or not. If it returns true, then reading stops.

A good example of this is to read a null-terminated string:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::ffi::CString;
# #[cfg(feature = "std")]
# #[derive(Debug, PartialEq, DekuRead)]
struct DekuTest {
    #[deku(until = "|v: &u8| *v == 0")]
    string: Vec<u8>
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[b'H', b'e', b'l', b'l', b'o', 0];
let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
        string: CString::new(b"Hello".to_vec()).unwrap().into_bytes_with_nul()
    },
    value
);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```
# read_all

Read values into the container until [reader.end()] returns `true`.

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct InnerDekuTest {
    field_a: u8,
    field_b: u8
}

# #[cfg(feature = "alloc")]
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(read_all)]
    items: Vec<InnerDekuTest>,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0xAB, 0xBC, 0xDE, 0xEF];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       items: vec![
           InnerDekuTest{field_a: 0xAB, field_b: 0xBC},
           InnerDekuTest{field_a: 0xDE, field_b: 0xEF}],
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(&*data, value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# update

Specify custom code to run on the field when `.update()` is called on the struct/enum

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
use core::convert::{TryInto, TryFrom};
use deku::prelude::*;
# #[cfg(feature = "alloc")]
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(update = "self.items.len()")]
    count: u8,
    #[deku(count = "count")]
    items: Vec<u8>,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0x02, 0xAB, 0xCD];

// `mut` so it can be updated
let mut value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { count: 0x02, items: vec![0xAB, 0xCD] },
    value
);

// push a new item to the vec
value.items.push(0xFF);

// update it, this will update the `count` field
value.update().unwrap();

assert_eq!(
    DekuTest { count: 0x03, items: vec![0xAB, 0xCD, 0xFF] },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x03, 0xAB, 0xCD, 0xFF], value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# temp

A temporary field

Included in the reading of the struct/enum but not stored

**Note**: Struct/enum must be derived with `#[deku_derive(...)]` to derive
`DekuRead` and/or `DekuWrite`, not with `#[derive(...)]`. This is because the
struct/enum needs to be modified at compile time.

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "alloc")]
#[deku_derive(DekuRead, DekuWrite)]
#[derive(Debug, PartialEq)]
struct DekuTest {
    #[deku(temp)]
    num_items: u8,

    #[deku(count = "num_items", endian = "big")]
    items: Vec<u16>,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0x01, 0xBE, 0xEF];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
       items: vec![0xBEEF]
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0xBE, 0xEF], value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```


# temp_value

Value for temporary field

Will be written on corresponding offset of the struct/enum

**Note**: Struct/enum must be derived with `#[deku_derive(...)]` to derive
`DekuRead` and/or `DekuWrite`, not with `#[derive(...)]`. This is because the
struct/enum needs to be modified at compile time.

Example:
```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "alloc")]
#[deku_derive(DekuRead, DekuWrite)]
#[derive(Debug, PartialEq)]
struct DekuTest {
    #[deku(temp, temp_value = "items.len() as u8")]
    num_items: u8,

    #[deku(count = "num_items", endian = "big")]
    items: Vec<u16>,
}

# #[cfg(feature = "alloc")]
# fn main() {
let value = DekuTest {
    items: vec![0xDEAD, 0xBEEF]
};
let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x02, 0xDE, 0xAD, 0xBE, 0xEF], value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# skip

Skip the reading/writing of a field.

Defaults value to [default](#default)

**Note**: Can be paired with [cond](#cond) to have conditional skipping

Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,
    #[deku(skip)]
    field_b: Option<u8>,
    field_c: u8,
}

let data: &[u8] = &[0x01, 0x02];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { field_a: 0x01, field_b: None, field_c: 0x02 },
    value
);
```

# pad_bytes_before

Skip a number of bytes before reading, pad with 0x00s before writing

Example:

```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "alloc")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct DekuTest {
    pub field_a: u8,
    #[deku(pad_bytes_before = "2")]
    pub field_b: u8,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0xAA, 0xBB, 0xCC, 0xDD];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
        field_a: 0xAA,
        field_b: 0xDD,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0xAA, 0x00, 0x00, 0xDD], value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# pad_bits_before

Skip a number of bytes before reading, pad with 0s before writing

Example:

```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "bits")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bits = 2)]
    field_a: u8,
    #[deku(pad_bits_before = "2", bits = 4)]
    field_b: u8,
}

# #[cfg(all(feature = "alloc", feature = "bits"))]
# fn main() {
let data: &[u8] = &[0b10_01_1001];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
        field_a: 0b10,
        field_b: 0b1001,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0b10_00_1001], value);
# }
#
# #[cfg(not(all(feature = "alloc", feature = "bits")))]
# fn main() {}
```

# pad_bytes_after

Skip a number of bytes after reading, pad with 0x00s after writing

Example:

```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "alloc")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
pub struct DekuTest {
    #[deku(pad_bytes_after = "2")]
    pub field_a: u8,
    pub field_b: u8,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0xAA, 0xBB, 0xCC, 0xDD];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
        field_a: 0xAA,
        field_b: 0xDD,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0xAA, 0x00, 0x00, 0xDD], value);
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# pad_bits_after

Skip a number of bytes after reading, pad with 0s after writing

Example:

```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "bits")]
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bits = 2, pad_bits_after = "2")]
    field_a: u8,
    #[deku(bits = 4)]
    field_b: u8,
}

# #[cfg(all(feature = "alloc", feature = "bits"))]
# fn main() {
let data: &[u8] = &[0b10_01_1001];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
        field_a: 0b10,
        field_b: 0b1001,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0b10_00_1001], value);
# }
#
# #[cfg(not(all(feature = "alloc", feature = "bits")))]
# fn main() {}
```

# cond

Specify a condition to parse or skip a field

**Note**: Can be paired with [default](#default)

Example:

```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::{vec, vec::Vec};
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,
    #[deku(cond = "*field_a == 0x01")]
    field_b: Option<u8>,
    #[deku(cond = "*field_b == Some(0xFF)", default = "Some(0x05)")]
    field_c: Option<u8>,
    #[deku(skip, cond = "*field_a == 0x01", default = "Some(0x06)")]
    field_d: Option<u8>,
}

# #[cfg(feature = "alloc")]
# fn main() {
let data: &[u8] = &[0x01, 0x02];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { field_a: 0x01, field_b: Some(0x02), field_c: Some(0x05), field_d: Some(0x06)},
    value
);

assert_eq!(
    vec![0x01, 0x02, 0x05],
    value.to_bytes().unwrap(),
)
# }
#
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

# default

Default code tokens used with [skip](#skip) or [cond](#cond)

Defaults to `Default::default()`

Example:

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,
    #[deku(skip, default = "Some(*field_a)")]
    field_b: Option<u8>,
    field_c: u8,
}

let data: &[u8] = &[0x01, 0x02];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { field_a: 0x01, field_b: Some(0x01), field_c: 0x02 },
    value
);
```

# map

Specify a function or lambda to apply to the result of the read

Example:

Read a `u8` and apply a function to convert it to a `String`.

```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
#[derive(PartialEq, Debug, DekuRead)]
struct DekuTest {
    #[deku(map = "|field: u8| -> Result<_, DekuError> { Ok(field.to_string()) }")]
    field_a: String,
    #[deku(map = "DekuTest::map_field_b")]
    field_b: String,
}

impl DekuTest {
    fn map_field_b(field_b: u8) -> Result<String, DekuError> {
        Ok(field_b.to_string())
    }
}

let data: &[u8] = &[0x01, 0x02];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { field_a: "1".to_string(), field_b: "2".to_string() },
    value
);
```

# reader/writer

Specify custom reader or writer tokens for reading a field or variant

Example:
```rust
use core::convert::{TryInto, TryFrom};
# #[cfg(all(feature = "alloc", feature = "bits"))]
use deku::bitvec::{BitSlice, BitVec, Msb0};
use deku::prelude::*;

# #[cfg(feature = "std")]
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(
        reader = "DekuTest::read(deku::reader)",
        writer = "DekuTest::write(deku::writer, &self.field_a)"
    )]
    field_a: String,
}

# #[cfg(feature = "std")]
impl DekuTest {
    /// Read and convert to String
    fn read<R: std::io::Read + std::io::Seek>(
        reader: &mut deku::reader::Reader<R>,
    ) -> Result<String, DekuError> {
        let value = u8::from_reader_with_ctx(reader, ())?;
        Ok(value.to_string())
    }

    /// Parse from String to u8 and write
    fn write<W: std::io::Write + std::io::Seek>(writer: &mut Writer<W>, field_a: &str) -> Result<(), DekuError> {
        let value = field_a.parse::<u8>().unwrap();
        value.to_writer(writer, ())
    }
}

# #[cfg(all(feature = "bits", feature = "std"))]
# fn main() {
let data: &[u8] = &[0x01];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest { field_a: "1".to_string() },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, &*value);
# }
#
# #[cfg(not(all(feature = "bits", feature = "std")))]
# fn main() {}
```

# ctx

This attribute allows sending and receiving context (variables/values) to sub-parsers/writers

**Note**: `endian`, `bytes`, `bits`, `count` attributes use `ctx` internally, see examples below

**top-level**: The value of a ctx attribute is a function argument list,
for example `#[deku(ctx = "a: u8, b: String")]`

**field-level**: The value of the ctx attribute is a list of expressions,
for example `#[deku("a, b")]`

**Visibility**: The following can be accessed:
1. All former fields which have been parsed (given as a reference).
2. `endian`, `bytes`, `bits` attributes declared on the top-level
    - These are prepended to the list of ctx variables

**Note**: The `enum` or `struct` that uses `ctx` will not implement [DekuContainerRead](crate::DekuContainerRead) or [DekuContainerWrite](crate::DekuContainerWrite) unless [ctx_default](#ctx_default) is also used.

Example
```rust
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(DekuRead, DekuWrite)]
#[deku(ctx = "a: u8")]
struct Subtype {
    #[deku(map = "|b: u8| -> Result<_, DekuError> { Ok(b + a) }")]
    b: u8
}

#[derive(DekuRead, DekuWrite)]
struct Test {
    a: u8,
    #[deku(ctx = "*a")] // pass `a` to `SubType`, `a` is a reference
    sub: Subtype
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0x02];
let mut cursor = Cursor::new(data);

let (amt_read, value) = Test::from_reader((&mut cursor, 0)).unwrap();
assert_eq!(value.a, 0x01);
assert_eq!(value.sub.b, 0x01 + 0x02)
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

**Note**: In addition, `endian`, `bytes` and `bits` use the `ctx` concept internally, examples below are equivalent:

Example:
```ignore
struct Type1 {
    #[deku(endian = "big", bits = 1)]
    field: u8,
}

// is equivalent to

struct Type1 {
    #[deku(ctx = "Endian::Big, BitSize(1)")]
    field: u8,
}
```

Example: Adding context
```ignore
#[deku(endian = "big")]
struct Type1 {
    field_a: u16,
    #[deku(bits = 5, ctx = "*field_a")]
    field_b: SubType,
}

// is equivalent to

struct Type1 {
    #[deku(ctx = "Endian::Big")]
    field_a: u16,
    #[deku(ctx = "Endian::Big, BitSize(5), *field_a")] // endian is prepended
    field_b: SubType,
}
```

# ctx_default

When paired with the [`ctx`](#ctx) attribute, `ctx_default` provides default
values for the context

Example:
```rust
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(DekuRead, DekuWrite)]
#[deku(ctx = "a: u8", ctx_default = "1")] // Defaults `a` to 1
struct Subtype {
    #[deku(map = "|b: u8| -> Result<_, DekuError> { Ok(b + a) }")]
    b: u8
}

#[derive(DekuRead, DekuWrite)]
struct Test {
    a: u8,
    #[deku(ctx = "*a")] // pass `a` to `SubType`, `a` is a reference
    sub: Subtype
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0x02];
let mut cursor = Cursor::new(data);

// Use with context from `Test`
let (amt_read, value) = Test::from_reader((&mut cursor, 0)).unwrap();
assert_eq!(value.a, 0x01);
assert_eq!(value.sub.b, 0x01 + 0x02);

// Use as a stand-alone container, using defaults
// Note: `from_reader` is now available on `SubType`
let data: &[u8] = &[0x02];
let mut cursor = Cursor::new(data);

let (amt_read, value) = Subtype::from_reader((&mut cursor, 0)).unwrap();
assert_eq!(value.b, 0x01 + 0x02)
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

# id

## id (top-level)

Specify the enum id

This is useful in cases when the enum `id` is already consumed or is given externally

Example:

```rust
# #[cfg(feature = "alloc")]
# extern crate alloc;
# #[cfg(feature = "alloc")]
# use alloc::vec::Vec;
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    my_id: u8,
    data: u8,
    #[deku(ctx = "*my_id")]
    enum_from_id: MyEnum,
}

#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(ctx = "my_id: u8", id = "my_id")]
enum MyEnum {
    #[deku(id = 1)]
    VariantA(u8),
    #[deku(id = 2)]
    VariantB,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01_u8, 0xff, 0xab];
let ret_read = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest {
        my_id: 0x01,
        data: 0xff,
        enum_from_id: MyEnum::VariantA(0xab),
    },
    ret_read
);

let ret_write: Vec<u8> = ret_read.try_into().unwrap();
assert_eq!(&*ret_write, data)
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

## id (variant)

Specify the identifier of the enum variant, must be paired with [id_type](#id_type)
or [id (top-level)](#id-top-level)

**Note**:
    - If no `id` is specified, it is defaulted to the discriminant value.
    - The discriminant value is retrieved using the `as` keyword.

Example:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum DekuTest {
    #[deku(id = 0x01)]
    VariantA(u8),
    #[deku(id = 0x02)]
    VariantB(u8, u16),
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0xFF, 0x02, 0xAB, 0xEF, 0xBE];
let mut cursor = Cursor::new(data);

let (amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest::VariantA(0xFF),
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x01, 0xFF], variant_bytes);

let (amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest::VariantB(0xAB, 0xBEEF),
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x02, 0xAB, 0xEF, 0xBE], variant_bytes);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

Example discriminant
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[repr(u8)]
#[deku(id_type = "u8")]
enum DekuTest {
    VariantA = 0x01,
    VariantB,
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x01, 0x02];
let mut cursor = Cursor::new(data);

let (amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest::VariantA,
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x01], variant_bytes);

let (rest, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest::VariantB,
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x02], variant_bytes);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

# id_endian

Specify the endianness of the variant `id`, without mandating the same endianness for the fields.

Example:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u16", id_endian = "big", endian = "little")]
enum DekuTest {
    // Takes its endianness from the enum spec
    #[deku(id = "0x01")]
    VariantLittle(u16),
    // Force the endianness on the field
    #[deku(id = "0x02")]
    VariantBig {
        #[deku(endian = "big")]
        x: u16,
    },
}

let data: Vec<u8> = vec![0x00, 0x01, 0x01, 0x00];

let (_, value) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();

assert_eq!(
    DekuTest::VariantLittle(1),
    value
);

// ID changes, data bytes the same
let data: Vec<u8> = vec![0x00, 0x02, 0x01, 0x00];

let (_, value) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();

assert_eq!(
    DekuTest::VariantBig { x: 256 },
    value
);
```

# id_pat

Specify the identifier in the form of a match pattern for the enum variant.

The first field of the variant may be used for storage, and must be the same type as `id_type` and no attributes.

If no storage for the id is provided, the enum discriminent (if provided) will be used to write as the id for that variant

The writing of the field will use the same options as the reading.

Example:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
enum DekuTest {
    #[deku(id = 0x01)]
    VariantA(u8),
    #[deku(id_pat = "0x02..=0x06")]
    VariantB {
        id: u8
    },
    #[deku(id_pat = "_")]
    VariantC(u8),
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0x03, 0xFF];
let mut cursor = Cursor::new(data);

let (amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest::VariantB { id: 0x03 },
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x03], variant_bytes);

let (rest, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest::VariantC(0xFF),
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0xFF], variant_bytes);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

# id_type

Specify the type of the enum variant id to consume, see [example](#id-variant)

# bits

Set the bit size of the enum variant `id`

**Note**: Cannot be used in combination with [bytes](#bytes)

Example:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
# #[cfg(feature = "bits")]
#[deku(id_type = "u8", bits = 4)]
enum DekuTest {
    #[deku(id = 0b1001)]
    VariantA( #[deku(bits = 4)] u8, u8),
}

# #[cfg(all(feature = "bits", feature = "std"))]
# fn main() {
let data: &[u8] = &[0b1001_0110, 0xFF];
let mut cursor = Cursor::new(data);

let (amt_read, value) = DekuTest::from_reader((&mut cursor, 0)).unwrap();

assert_eq!(
    DekuTest::VariantA(0b0110, 0xFF),
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
# }
#
# #[cfg(not(all(feature = "bits", feature = "std")))]
# fn main() {}
```


When using `id_type` with non-unit variants, Deku requires [primitive representation](https://doc.rust-lang.org/reference/type-layout.html#primitive-representations).
Deku uses this for calculating the discriminant when reading and writing.
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[cfg(feature = "std")]
# use std::io::Cursor;
#[derive(Copy, Clone, Debug, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian", id_type = "u8")]
#[repr(u8)]
enum OpKind {
    Id = 0,
    Opcode = 255,
}
```



# bytes

Set the byte size of the enum variant `id`

**Note**: Cannot be used in combination with [bits](#bits)

Example:
```rust
# use core::convert::{TryInto, TryFrom};
# use deku::prelude::*;
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u32", bytes = 2)]
enum DekuTest {
    #[deku(id = 0xBEEF)]
    VariantA(u8),
}

# #[cfg(feature = "std")]
# fn main() {
let data: &[u8] = &[0xEF, 0xBE, 0xFF];

let value = DekuTest::try_from(data).unwrap();

assert_eq!(
    DekuTest::VariantA(0xFF),
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
# }
#
# #[cfg(not(feature = "std"))]
# fn main() {}
```

[reader.end()]: crate::reader::Reader::end()
*/
