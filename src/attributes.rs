/*!
A documentation-only module for #\[deku\] attributes

# List of attributes

| Attribute | Scope | Description
|-----------|------------------|------------
| [endian](#endian) | top-level, field | Set the endianness
| [magic](#magic) | top-level | A magic value that must be present at the start of this struct/enum
| [bits](#bits) | field | Set the bit-size of the field
| [bytes](#bytes) | field | Set the byte-size of the field
| [count](#count) | field | Set the field representing the element count of a container
| [bits_read](#bits_read) | field | Set the field representing the number of bits to read into a container
| [bytes_read](#bytes_read) | field | Set the field representing the number of bytes to read into a container
| [until](#until) | field | Set a predicate returning when to stop reading elements into a container
| [update](#update) | field | Apply code over the field when `.update()` is called
| [temp](#temp) | field | Read the field but exclude it from the struct/enum
| [skip](#skip) | field | Skip the reading/writing of a field
| [cond](#cond) | field | Conditional expression for the field
| [default](#default) | field | Custom defaulting code when `skip` is true
| [map](#map) | field | Apply a function over the result of reading
| [reader](#readerwriter) | variant, field | Custom reader code
| [writer](#readerwriter) | variant, field | Custom writer code
| [ctx](#ctx) | top-level, field| Context list for context sensitive parsing
| [ctx_default](#ctx_default) | top-level, field| Default context values
| enum: [id](#id) | top-level, variant | enum or variant id value
| enum: [id_pat](#id_pat) | variant | variant id match pattern
| enum: [type](#type) | top-level | Set the type of the variant `id`
| enum: [bits](#bits) | top-level | Set the bit-size of the variant `id`
| enum: [bytes](#bytes) | top-level | Set the byte-size of the variant `id`

# endian

Set to read/write bytes in a specific byte order.

Values: `big`, `little` or an expression which returns a [`Endian`](super::ctx::Endian)

Precedence: field > top-level > system endianness (default)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
// #[deku(endian = "little")] // top-level, defaults to system endianness
struct DekuTest {
    #[deku(endian = "big")] // field-level override
    field_be: u16,
    field_default: u16, // defaults to top-level
}

let data: Vec<u8> = vec![0xAB, 0xCD, 0xAB, 0xCD];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest {
       field_be: 0xABCD,
       field_default: 0xCDAB,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
```

**Note**: The `endian` is passed as a context argument to sub-types

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")] // context passed from `DekuTest` top-level endian
struct Child {
    field_a: u16
}

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

let data: Vec<u8> = vec![0xAB, 0xCD, 0xAB, 0xCD, 0xEF, 0xBE];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest {
       field_be: 0xABCD,
       field_default: 0xCDAB,
       field_child: Child { field_a: 0xBEEF }
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
```

# magic

Sets a "magic" value that must be present in the data at the start of
a struct/enum when reading, and that is written out of the start of
that type's data when writing.

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(magic = b"deku")]
struct DekuTest {
    data: u8
}

let data: Vec<u8> = vec![b'd', b'e', b'k', b'u', 50];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest { data: 50 },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
```

# bits

Set the bit-size of the field

**Note**: Cannot be used in combination with [bytes](#bytes)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bits = 2)]
    field_a: u8,
    #[deku(bits = 6)]
    field_b: u8,
    field_c: u8, // defaults to size_of<u8>*8
}

let data: Vec<u8> = vec![0b11_101010, 0xFF];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest {
       field_a: 0b11,
       field_b: 0b101010,
       field_c: 0xFF,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
```

# bytes

Set the byte-size of the field

**Note**: Cannot be used in combination with [bits](#bits)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(bytes = 2)]
    field_a: u32,
    field_b: u8, // defaults to size_of<u8>
}

let data: Vec<u8> = vec![0xAB, 0xCD, 0xFF];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest {
       field_a: 0xCDAB,
       field_b: 0xFF,
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
```

# count

Specify the field representing the length of the container, i.e. a Vec

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(update = "self.items.len()")]
    count: u8,
    #[deku(count = "count")]
    items: Vec<u8>,
}

let data: Vec<u8> = vec![0x02, 0xAB, 0xCD];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest {
       count: 0x02,
       items: vec![0xAB, 0xCD],
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
```

**Note**: See [update](#update) for more information on the attribute!


# bytes_read

Specify the field representing the total number of bytes to read into a container

See the following example, where `InnerDekuTest` is 2 bytes, so setting `bytes_read` to
4 will read 2 items into the container:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct InnerDekuTest {
    field_a: u8,
    field_b: u8
}

# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(update = "(self.items.len() / 2)")]
    bytes: u8,

    #[deku(bytes_read = "bytes")]
    items: Vec<InnerDekuTest>,
}

let data: Vec<u8> = vec![0x04, 0xAB, 0xBC, 0xDE, 0xEF];

let value = DekuTest::try_from(data.as_ref()).unwrap();

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
assert_eq!(data, value);
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
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# use std::ffi::CString;
# #[derive(Debug, PartialEq, DekuRead)]
struct DekuTest {
    #[deku(until = "|v: &u8| *v == 0")]
    string: Vec<u8>
}

let data: Vec<u8> = vec![b'H', b'e', b'l', b'l', b'o', 0];
let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest {
        string: CString::new(b"Hello".to_vec()).unwrap().into_bytes_with_nul()
    },
    value
);
```


# update

Specify custom code to run on the field when `.update()` is called on the struct/enum

Example:
```rust
use deku::prelude::*;
use std::convert::{TryInto, TryFrom};
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(update = "self.items.len()")]
    count: u8,
    #[deku(count = "count")]
    items: Vec<u8>,
}

let data: Vec<u8> = vec![0x02, 0xAB, 0xCD];

// `mut` so it can be updated
let mut value = DekuTest::try_from(data.as_ref()).unwrap();

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
```

# temp

A temporary field

Included in the reading of the struct/enum but not stored

**Note**: Struct/enum must be derived with `#[deku_derive(...)]` to derive
`DekuRead` and/or `DekuWrite`, not with `#[derive(...)]`. This is because the
struct/enum needs to be modified at compile time.

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
#[deku_derive(DekuRead, DekuWrite)]
#[derive(Debug, PartialEq)]
struct DekuTest {
    #[deku(temp)]
    num_items: u8,

    #[deku(count = "num_items", endian = "big")]
    items: Vec<u16>,
}

let data: Vec<u8> = vec![0x01, 0xBE, 0xEF];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest {
       items: vec![0xBEEF]
    },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0xBE, 0xEF], value);
```

# skip

Skip the reading/writing of a field.

Defaults value to [default](#default)

**Note**: Can be paired with [cond](#cond) to have conditional skipping

Example:

```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,
    #[deku(skip)]
    field_b: Option<u8>,
    field_c: u8,
}

let data: Vec<u8> = vec![0x01, 0x02];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest { field_a: 0x01, field_b: None, field_c: 0x02 },
    value
);
```

# cond

Specify a condition to parse or skip a field

**Note**: Can be paired with [default](#default)

Example:

```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
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

let data: Vec<u8> = vec![0x01, 0x02];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest { field_a: 0x01, field_b: Some(0x02), field_c: Some(0x05), field_d: Some(0x06)},
    value
);

assert_eq!(
    vec![0x01, 0x02, 0x05],
    value.to_bytes().unwrap(),
)
```

# default

Default code tokens used with [skip](#skip) or [cond](#cond)

Defaults to `Default::default()`

Example:

```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
#[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    field_a: u8,
    #[deku(skip, default = "Some(*field_a)")]
    field_b: Option<u8>,
    field_c: u8,
}

let data: Vec<u8> = vec![0x01, 0x02];

let value = DekuTest::try_from(data.as_ref()).unwrap();

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
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
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

let data: Vec<u8> = vec![0x01, 0x02];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest { field_a: "1".to_string(), field_b: "2".to_string() },
    value
);
```

# reader/writer

Specify custom reader or writer tokens for reading a field or variant

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
struct DekuTest {
    #[deku(
        reader = "DekuTest::read(deku::rest)",
        writer = "DekuTest::write(deku::output, &self.field_a)"
    )]
    field_a: String,
}

impl DekuTest {
    /// Read and convert to String
    fn read(
        rest: &BitSlice<Msb0, u8>,
    ) -> Result<(&BitSlice<Msb0, u8>, String), DekuError> {
        let (rest, value) = u8::read(rest, ())?;
        Ok((rest, value.to_string()))
    }

    /// Parse from String to u8 and write
    fn write(output: &mut BitVec<Msb0, u8>, field_a: &str) -> Result<(), DekuError> {
        let value = field_a.parse::<u8>().unwrap();
        value.write(output, ())
    }
}

let data: Vec<u8> = vec![0x01];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest { field_a: "1".to_string() },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
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

Example
```rust
# use deku::prelude::*;
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

let data: Vec<u8> = vec![0x01, 0x02];

let (rest, value) = Test::from_bytes((&data[..], 0)).unwrap();
assert_eq!(value.a, 0x01);
assert_eq!(value.sub.b, 0x01 + 0x02)
```

**Note**: In addition, `endian`, `bytes` and `bits` use the `ctx` concept internally, examples below are equivalent:

Example:
```ignore
struct Type1 {
    #[deku(endian = "big", bits = "1")]
    field: u8,
}

// is equivalent to

struct Type1 {
    #[deku(ctx = "Endian::Big, Size::Bits(1)")]
    field: u8,
}
```

Example: Adding context
```ignore
#[deku(endian = "big")]
struct Type1 {
    field_a: u16,
    #[deku(bits = "5", ctx = "*field_a")]
    field_b: SubType,
}

// is equivalent to

struct Type1 {
    #[deku(ctx = "Endian::Big")]
    field_a: u16,
    #[deku(ctx = "Endian::Big, Size::Bits(5), *field_a")] // endian is prepended
    field_b: SubType,
}
```

# ctx_default

When paired with the [`ctx`](#ctx) attribute, `ctx_default` provides default
values for the context

Example:
```rust
# use deku::prelude::*;
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

let data: Vec<u8> = vec![0x01, 0x02];

// Use with context from `Test`
let (rest, value) = Test::from_bytes((&data[..], 0)).unwrap();
assert_eq!(value.a, 0x01);
assert_eq!(value.sub.b, 0x01 + 0x02);

// Use as a stand-alone container, using defaults
// Note: `from_bytes` is now available on `SubType`
let data: Vec<u8> = vec![0x02];

let (rest, value) = Subtype::from_bytes((&data[..], 0)).unwrap();
assert_eq!(value.b, 0x01 + 0x02)
```

# id

## id (top-level)

Specify the enum id

This is useful in cases when the enum `id` is already consumed or is given externally

Example:

```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
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
    #[deku(id = "1")]
    VariantA(u8),
    #[deku(id = "2")]
    VariantB,
}

let data: Vec<u8> = vec![0x01_u8, 0xff, 0xab];
let ret_read = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest {
        my_id: 0x01,
        data: 0xff,
        enum_from_id: MyEnum::VariantA(0xab),
    },
    ret_read
);

let ret_write: Vec<u8> = ret_read.try_into().unwrap();
assert_eq!(ret_write, data)
```

## id (variant)

Specify the identifier of the enum variant, must be paired with [type](#type)
or [id (top-level)](#id-top-level)

**Note**:
    - If no `id` is specified, it is defaulted to the discriminant value.
    - The discriminant value is retreived using the `as` keyword.

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(type = "u8")]
enum DekuTest {
    #[deku(id = "0x01")]
    VariantA(u8),
    #[deku(id = "0x02")]
    VariantB(u8, u16),
}

let data: Vec<u8> = vec![0x01, 0xFF, 0x02, 0xAB, 0xEF, 0xBE];

let (rest, value) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();

assert_eq!(
    DekuTest::VariantA(0xFF),
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x01, 0xFF], variant_bytes);

let (rest, value) = DekuTest::from_bytes(rest).unwrap();

assert_eq!(
    DekuTest::VariantB(0xAB, 0xBEEF),
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x02, 0xAB, 0xEF, 0xBE], variant_bytes);
```

Example discriminant
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(type = "u8")]
enum DekuTest {
    VariantA = 0x01,
    VariantB,
}

let data: Vec<u8> = vec![0x01, 0x02];

let (rest, value) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();

assert_eq!(
    DekuTest::VariantA,
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x01], variant_bytes);

let (rest, value) = DekuTest::from_bytes(rest).unwrap();

assert_eq!(
    DekuTest::VariantB,
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x02], variant_bytes);
```

# id_pat

Specify the identifier in the form of a match pattern for the enum variant.

The enum variant must have space to store the identifier for proper writing.

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(type = "u8")]
enum DekuTest {
    #[deku(id = "0x01")]
    VariantA(u8),
    #[deku(id_pat = "0x02..=0x06")]
    VariantB {
        id: u8
    },
    #[deku(id_pat = "_")]
    VariantC(u8),
}

let data: Vec<u8> = vec![0x03, 0xFF];

let (rest, value) = DekuTest::from_bytes((data.as_ref(), 0)).unwrap();

assert_eq!(
    DekuTest::VariantB { id: 0x03 },
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x03], variant_bytes);

let (rest, value) = DekuTest::from_bytes(rest).unwrap();

assert_eq!(
    DekuTest::VariantC(0xFF),
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0xFF], variant_bytes);
```

# type

Specify the type of the enum variant id to consume, see [example](#id-variant)

# bits

Set the bit size of the enum variant `id`

**Note**: Cannot be used in combination with [bytes](#bytes)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(type = "u8", bits = "4")]
enum DekuTest {
    #[deku(id = "0b1001")]
    VariantA( #[deku(bits = "4")] u8, u8),
}

let data: Vec<u8> = vec![0b1001_0110, 0xFF];

let (rest, value) = DekuTest::from_bytes((&data, 0)).unwrap();

assert_eq!(
    DekuTest::VariantA(0b0110, 0xFF),
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
```

# bytes

Set the byte size of the enum variant `id`

**Note**: Cannot be used in combination with [bits](#bits)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(type = "u32", bytes = "2")]
enum DekuTest {
    #[deku(id = "0xBEEF")]
    VariantA(u8),
}

let data: Vec<u8> = vec![0xEF, 0xBE, 0xFF];

let value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest::VariantA(0xFF),
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(data, value);
```


*/
