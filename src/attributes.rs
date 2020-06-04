/*!
A documentation-only module for #[deku] attributes

# List of attributes

| Attribute | Scope | Description
|-----------|------------------|------------
| [endian](#endian) | top-level, field | Set the endianness
| [bits](#bits) | field | Set the bit-size of the field
| [bytes](#bytes) | field | Set the byte-size of the field
| [count](#count) | field | Set the field representing the element count of a container
| [map](#map) | field | Apply a function over the result of reading
| [reader](#readerwriter) | variant, field | Custom reader code
| [writer](#readerwriter) | variant, field | Custom writer code
| [id](#id) | variant | variant id value, paired with `id_type`
| [id_type](#id_type) | top-level (enum only) | Set the type of the variant `id`
| [id_bits](#id_bits) | top-level (enum only) | Set the bit-size of the variant `id`
| [id_bytes](#id_bytes) | top-level (enum only) | Set the byte-size of the variant `id`

# endian

Set to read/write bytes in a specific byte order.

Values: `big` or `little`

Precedence: field > top-level > system endianness (default)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")] // top-level, defaults to system endianness
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

**Note**: calling `.update()` on a struct derived with `DekuWrite` will update the `count` field!

```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(Debug, PartialEq, DekuRead, DekuWrite)]
# struct DekuTest {
#     count: u8,
#     #[deku(count = "count")]
#     items: Vec<u8>,
# }
#
let data: Vec<u8> = vec![0x02, 0xAB, 0xCD];

let mut value = DekuTest::try_from(data.as_ref()).unwrap();

assert_eq!(
    DekuTest { count: 0x02, items: vec![0xAB, 0xCD] },
    value
);

value.items.push(0xFF); // new item!
value.update().unwrap();

assert_eq!(
    DekuTest { count: 0x03, items: vec![0xAB, 0xCD, 0xFF] },
    value
);

let value: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x03, 0xAB, 0xCD, 0xFF], value);
```

# map

Specify a function or lambda to apply to the result of the read

Example:

Read a `u8` and apply a function to convert it to a `String`.

```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
#[derive(PartialEq, Debug, DekuRead)]
pub struct DekuTest {
    #[deku(map = "|field: u8| -> Result<_, DekuError> { Ok(field.to_string()) }")]
    pub field_a: String,
    #[deku(map = "DekuTest::map_field_b")]
    pub field_b: String,
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
        reader = "DekuTest::read(rest, input_is_le, field_bits)",
        writer = "DekuTest::write(&self.field_a, output_is_le, field_bits)"
    )]
    field_a: String,
}

impl DekuTest {
    // Read and convert to String
    fn read(
        rest: &BitSlice<Msb0, u8>,
        input_is_le: bool,
        bit_size: Option<usize>,
    ) -> Result<(&BitSlice<Msb0, u8>, String), DekuError> {
        let (rest, value) = u8::read(rest, input_is_le, bit_size, None)?;
        Ok((rest, value.to_string()))
    }

    // Parse from String to u8 and write
    fn write(field_a: &str, output_is_le: bool, bit_size: Option<usize>) -> Result<BitVec<Msb0, u8>, DekuError> {
        let value = field_a.parse::<u8>().unwrap();
        value.write(output_is_le, bit_size)
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

# id

Specify the identifier of the enum variant, must be paired with [id_type](#id_type)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8")]
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
# assert_eq!(0, rest.0.len());
# assert_eq!(0, rest.1);

assert_eq!(
    DekuTest::VariantB(0xAB, 0xBEEF),
    value
);

let variant_bytes: Vec<u8> = value.try_into().unwrap();
assert_eq!(vec![0x02, 0xAB, 0xEF, 0xBE], variant_bytes);
```

# id_type

Specify the type of the enum variant id, see [example](#id)

# id_bits

Set the bit size of the enum variant `id`

**Note**: Cannot be used in combination with [id_bytes](#id_bytes)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u8", id_bits = "4")]
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

# id_bytes

Set the byte size of the enum variant `id`

**Note**: Cannot be used in combination with [id_bits](#id_bits)

Example:
```rust
# use deku::prelude::*;
# use std::convert::{TryInto, TryFrom};
# #[derive(PartialEq, Debug, DekuRead, DekuWrite)]
#[deku(id_type = "u32", id_bytes = "2")]
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
