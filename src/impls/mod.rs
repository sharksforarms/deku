#[cfg(feature = "alloc")]
use no_std_io::io::{Read, Seek};

#[cfg(feature = "alloc")]
use crate::DekuError;
#[cfg(feature = "alloc")]
use crate::ctx::Limit;
#[cfg(feature = "alloc")]
use crate::reader::Reader;

#[cfg(feature = "alloc")]
trait ReadCollection<Item>: Sized {
    fn with_capacity(capacity: Option<usize>) -> Self;

    fn insert_item(&mut self, item: Item);
}

#[cfg(feature = "alloc")]
fn read_collection_with_predicate<'a, R, Item, Collection, Ctx, Predicate, Parse>(
    reader: &mut Reader<'a, R>,
    capacity: Option<usize>,
    ctx: Ctx,
    mut parse: Parse,
    mut predicate: Predicate,
) -> Result<Collection, DekuError>
where
    R: Read + Seek,
    Collection: ReadCollection<Item>,
    Ctx: Copy,
    Predicate: FnMut(usize, &Item) -> bool,
    Parse: FnMut(&mut Reader<'a, R>, Ctx) -> Result<Item, DekuError>,
{
    let mut result = Collection::with_capacity(capacity);
    let start_bits = reader.bits_read;

    loop {
        let item = parse(reader, ctx)?;
        let stop = predicate(reader.bits_read - start_bits, &item);
        result.insert_item(item);

        if stop {
            break;
        }
    }

    Ok(result)
}

#[cfg(feature = "alloc")]
fn read_collection_to_end<'a, R, Item, Collection, Ctx, Parse>(
    reader: &mut Reader<'a, R>,
    capacity: Option<usize>,
    ctx: Ctx,
    mut parse: Parse,
) -> Result<Collection, DekuError>
where
    R: Read + Seek,
    Collection: ReadCollection<Item>,
    Ctx: Copy,
    Parse: FnMut(&mut Reader<'a, R>, Ctx) -> Result<Item, DekuError>,
{
    let mut result = Collection::with_capacity(capacity);

    while !reader.end() {
        let item = parse(reader, ctx)?;
        result.insert_item(item);
    }

    Ok(result)
}

#[cfg(feature = "alloc")]
fn read_collection_with_limit<'a, R, Item, Collection, Ctx, Predicate, Parse>(
    reader: &mut Reader<'a, R>,
    limit: Limit<Item, Predicate>,
    ctx: Ctx,
    mut parse: Parse,
) -> Result<Collection, DekuError>
where
    R: Read + Seek,
    Collection: ReadCollection<Item>,
    Ctx: Copy,
    Predicate: FnMut(&Item) -> bool,
    Parse: FnMut(&mut Reader<'a, R>, Ctx) -> Result<Item, DekuError>,
{
    match limit {
        Limit::Count(mut count) => {
            if count == 0 {
                return Ok(Collection::with_capacity(None));
            }

            read_collection_with_predicate(reader, Some(count), ctx, &mut parse, move |_, _| {
                count -= 1;
                count == 0
            })
        }
        Limit::Until(mut predicate, _) => {
            read_collection_with_predicate(reader, None, ctx, &mut parse, move |_, item| {
                predicate(item)
            })
        }
        Limit::BitSize(size) => {
            let bit_size = size.0;
            if bit_size == 0 {
                return Ok(Collection::with_capacity(None));
            }

            read_collection_with_predicate(reader, None, ctx, &mut parse, move |read_bits, _| {
                read_bits == bit_size
            })
        }
        Limit::ByteSize(size) => {
            let bit_size = size.0 * 8;
            if bit_size == 0 {
                return Ok(Collection::with_capacity(None));
            }

            read_collection_with_predicate(reader, None, ctx, &mut parse, move |read_bits, _| {
                read_bits == bit_size
            })
        }
        Limit::End => read_collection_to_end(reader, None, ctx, &mut parse),
    }
}

mod bool;
mod ipaddr;
mod nonzero;
mod option;
mod primitive;
mod slice;
mod tuple;
mod unit;

#[cfg(feature = "alloc")]
mod vec;

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
mod arc;

#[cfg(feature = "alloc")]
mod cow;

#[cfg(feature = "alloc")]
mod cstring;

#[cfg(feature = "std")]
mod hashmap;

#[cfg(feature = "std")]
mod hashset;

#[cfg(feature = "alloc")]
mod boxed;
