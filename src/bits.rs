//! Small packed bit storage used by the `bits` feature.

use core::fmt;
use core::marker::PhantomData;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Marker retained for callers that used the old bit-order type in macros.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Msb0;

/// Marker retained for callers that used the old bit-order type in macros.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Lsb0;

/// Bit ordering used when converting a packed vector to raw bytes.
pub trait BitOrder {
    /// Encode the canonical, MSB-first bits in `bytes` using this order.
    fn encode(bytes: &mut [u8]);

    /// Decode bytes stored using this order into canonical MSB-first bits.
    #[inline]
    fn decode(bytes: &mut [u8]) {
        Self::encode(bytes);
    }
}

impl BitOrder for Msb0 {
    #[inline]
    fn encode(_: &mut [u8]) {}
}

impl BitOrder for Lsb0 {
    #[inline]
    fn encode(bytes: &mut [u8]) {
        for byte in bytes {
            *byte = byte.reverse_bits();
        }
    }
}

/// An immutable view over packed, MSB-first bits.
///
/// Bit zero is the high bit of the first byte. The view may start and end at
/// arbitrary bit offsets without allocating. `T` and `O` are compatibility
/// markers and do not change how the view interprets bits.
pub struct BitSlice<'a, T = u8, O = Msb0> {
    bytes: &'a [u8],
    offset: usize,
    len: usize,
    _marker: PhantomData<(T, O)>,
}

impl<'a, T, O> Copy for BitSlice<'a, T, O> {}

impl<'a, T, O> Clone for BitSlice<'a, T, O> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, O> BitSlice<'a, T, O> {
    /// View all bits in `bytes`.
    ///
    /// # Panics
    /// Panics if `bytes.len() * 8` cannot be represented by `usize`.
    #[inline]
    pub fn from_slice(bytes: &'a [u8]) -> Self {
        Self::from_parts(bytes, 0, bit_len_from_bytes(bytes.len()))
    }

    /// Create an empty bit view.
    #[inline]
    pub const fn empty() -> BitSlice<'static, T, O> {
        BitSlice {
            bytes: &[],
            offset: 0,
            len: 0,
            _marker: PhantomData,
        }
    }

    /// Create a view over a bit range in `bytes`.
    #[inline]
    pub(crate) fn from_parts(bytes: &'a [u8], offset: usize, len: usize) -> Self {
        let end = offset.checked_add(len).expect("bit range overflow");
        assert!(end <= bytes.len().saturating_mul(8));
        Self {
            bytes,
            offset,
            len,
            _marker: PhantomData,
        }
    }

    /// Number of bits in this view.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether this view contains no bits.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the bit at `index`.
    #[inline]
    pub fn get(&self, index: usize) -> bool {
        assert!(index < self.len);
        bit_at(self.bytes, self.offset + index)
    }

    /// Return a sub-view with the half-open range `[start, end]`.
    #[inline]
    pub fn subslice(&self, start: usize, end: usize) -> Self {
        assert!(start <= end);
        assert!(end <= self.len);
        Self::from_parts(self.bytes, self.offset + start, end - start)
    }

    /// Split this view at `mid`.
    #[inline]
    pub fn split_at(&self, mid: usize) -> (Self, Self) {
        (self.subslice(0, mid), self.subslice(mid, self.len))
    }

    /// Iterate over consecutive chunks, including a final partial chunk.
    ///
    /// # Panics
    /// Panics if `chunk_len` is zero.
    #[inline]
    pub fn chunks(&self, chunk_len: usize) -> BitChunks<'a, T, O> {
        assert!(chunk_len != 0);
        BitChunks {
            slice: *self,
            chunk_len,
            index: 0,
        }
    }

    /// Iterate over the bits in serialization order.
    #[inline]
    pub fn iter(&self) -> BitIter<'a, T, O> {
        BitIter {
            slice: *self,
            index: 0,
        }
    }

    /// Find the first set bit.
    #[inline]
    pub fn first_one(&self) -> Option<usize> {
        self.iter().position(|bit| bit)
    }

    /// Find the first clear bit.
    #[inline]
    pub fn first_zero(&self) -> Option<usize> {
        self.iter().position(|bit| !bit)
    }

    /// Find the last set bit.
    #[inline]
    pub fn last_one(&self) -> Option<usize> {
        self.iter().rposition(|bit| bit)
    }

    /// Find the last clear bit.
    #[inline]
    pub fn last_zero(&self) -> Option<usize> {
        self.iter().rposition(|bit| !bit)
    }

    /// Return the underlying bytes when this view is byte-aligned.
    #[inline]
    pub(crate) fn aligned_bytes(&self) -> Option<&'a [u8]> {
        if !self.offset.is_multiple_of(8) || !self.len.is_multiple_of(8) {
            return None;
        }
        let start = self.offset / 8;
        let end = start + self.len / 8;
        Some(&self.bytes[start..end])
    }

    /// Load this bit sequence as a big-endian integer.
    ///
    /// # Panics
    /// Panics if the bit sequence is wider than `V::BITS`, or if `V::BITS` is
    /// greater than 128.
    #[inline]
    pub fn load_be<V: BitValue>(&self) -> V {
        assert_bit_value_len::<V>(self.len);
        if let Some(bytes) = self.aligned_bytes() {
            return V::from_u128(load_be_bytes(bytes, 0, bytes.len()));
        }
        if self.len == 0 {
            return V::from_u128(0);
        }
        if self.len <= 8 {
            return V::from_u128(u128::from(load_be_chunk(self.bytes, self.offset, self.len)));
        }
        V::from_u128(load_be_unaligned(self.bytes, self.offset, self.len))
    }

    /// Load this bit sequence as a little-endian integer.
    ///
    /// # Panics
    /// Panics if the bit sequence is wider than `V::BITS`, or if `V::BITS` is
    /// greater than 128.
    #[inline]
    pub fn load_le<V: BitValue>(&self) -> V {
        assert_bit_value_len::<V>(self.len);
        if let Some(bytes) = self.aligned_bytes() {
            let mut value = 0u128;
            for &byte in bytes.iter().rev() {
                value = (value << 8) | u128::from(byte);
            }
            return V::from_u128(value);
        }
        if self.len == 0 {
            return V::from_u128(0);
        }
        V::from_u128(load_le_unaligned(self.bytes, self.offset, self.len))
    }

    /// Load this bit sequence using its packed MSB-first representation.
    #[inline]
    pub fn load<V: BitValue>(&self) -> V {
        self.load_be()
    }
}

impl<T, O> fmt::Debug for BitSlice<'_, T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BitSlice").field("len", &self.len).finish()
    }
}

impl<T, O, U, P> PartialEq<BitSlice<'_, U, P>> for BitSlice<'_, T, O> {
    fn eq(&self, other: &BitSlice<'_, U, P>) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}

impl<T, O> Eq for BitSlice<'_, T, O> {}

/// A read-only source that can be written as packed MSB-first bits.
pub trait BitSource {
    /// Borrow the source as a packed bit view.
    fn bit_slice(&self) -> BitSlice<'_, u8, Msb0>;
}

impl<T, O> BitSource for BitSlice<'_, T, O> {
    #[inline]
    fn bit_slice(&self) -> BitSlice<'_, u8, Msb0> {
        BitSlice::from_parts(self.bytes, self.offset, self.len)
    }
}

impl BitSource for [u8] {
    #[inline]
    fn bit_slice(&self) -> BitSlice<'_, u8, Msb0> {
        BitSlice::from_slice(self)
    }
}

/// Iterator over a [`BitSlice`].
#[derive(Clone, Copy)]
pub struct BitIter<'a, T = u8, O = Msb0> {
    slice: BitSlice<'a, T, O>,
    index: usize,
}

impl<T, O> Iterator for BitIter<'_, T, O> {
    type Item = bool;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.slice.len {
            None
        } else {
            let bit = self.slice.get(self.index);
            self.index += 1;
            Some(bit)
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.slice.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<T, O> DoubleEndedIterator for BitIter<'_, T, O> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index == self.slice.len {
            None
        } else {
            let index = self.slice.len - 1;
            let bit = self.slice.get(index);
            self.slice.len = index;
            Some(bit)
        }
    }
}

impl<T, O> ExactSizeIterator for BitIter<'_, T, O> {}

impl<T, O> BitIter<'_, T, O> {
    /// Compatibility helper for callers that previously used bitvec's
    /// reference-to-value iterator adapter.
    #[inline]
    pub fn by_vals(self) -> Self {
        self
    }
}

/// Iterator returned by [`BitSlice::chunks`].
#[derive(Clone, Copy)]
pub struct BitChunks<'a, T = u8, O = Msb0> {
    slice: BitSlice<'a, T, O>,
    chunk_len: usize,
    index: usize,
}

impl<'a, T, O> Iterator for BitChunks<'a, T, O> {
    type Item = BitSlice<'a, T, O>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.slice.len() {
            return None;
        }
        let end = chunk_end(self.index, self.slice.len(), self.chunk_len);
        let chunk = self.slice.subslice(self.index, end);
        self.index = end;
        Some(chunk)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.slice.len().saturating_sub(self.index);
        let count = remaining.div_ceil(self.chunk_len);
        (count, Some(count))
    }
}

/// A mutable view over packed, MSB-first bits.
pub struct BitSliceMut<'a, T = u8, O = Msb0> {
    bytes: &'a mut [u8],
    offset: usize,
    len: usize,
    _marker: PhantomData<(T, O)>,
}

impl<'a, T, O> BitSliceMut<'a, T, O> {
    /// View all bits in `bytes`.
    ///
    /// # Panics
    /// Panics if `bytes.len() * 8` cannot be represented by `usize`.
    #[inline]
    pub fn from_slice(bytes: &'a mut [u8]) -> Self {
        let len = bit_len_from_bytes(bytes.len());
        Self::from_parts(bytes, 0, len)
    }

    /// Create a mutable view over a bit range in `bytes`.
    #[inline]
    pub(crate) fn from_parts(bytes: &'a mut [u8], offset: usize, len: usize) -> Self {
        let end = offset.checked_add(len).expect("bit range overflow");
        assert!(end <= bytes.len().saturating_mul(8));
        Self {
            bytes,
            offset,
            len,
            _marker: PhantomData,
        }
    }

    /// Number of bits in this view.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether this view contains no bits.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return a mutable sub-view with the half-open range `[start, end]`.
    #[inline]
    pub fn subslice(&mut self, start: usize, end: usize) -> BitSliceMut<'_, T, O> {
        assert!(start <= end);
        assert!(end <= self.len);
        BitSliceMut::from_parts(&mut *self.bytes, self.offset + start, end - start)
    }

    /// Return the bit at `index`.
    #[inline]
    pub fn get(&self, index: usize) -> bool {
        assert!(index < self.len);
        bit_at(self.bytes, self.offset + index)
    }

    /// Set the bit at `index`.
    #[inline]
    pub fn set(&mut self, index: usize, value: bool) {
        assert!(index < self.len);
        set_bit(self.bytes, self.offset + index, value);
    }

    /// Copy all bits from `source` into this view.
    #[inline]
    pub fn copy_from_bitslice<U, P>(&mut self, source: &BitSlice<'_, U, P>) {
        assert!(source.len() <= self.len);
        self.copy_from_bitslice_at(0, source);
    }

    /// Copy `source` into this view beginning at `start`.
    #[inline]
    pub fn copy_from_bitslice_at<U, P>(&mut self, start: usize, source: &BitSlice<'_, U, P>) {
        assert!(start.checked_add(source.len()).is_some());
        assert!(start + source.len() <= self.len);

        let destination_start = self.offset + start;
        if destination_start.is_multiple_of(8)
            && let Some(source_bytes) = source.aligned_bytes()
        {
            let destination_start = destination_start / 8;
            let destination_end = destination_start + source_bytes.len();
            self.bytes[destination_start..destination_end].copy_from_slice(source_bytes);
            return;
        }

        let mut copied = 0;
        while copied < source.len() {
            let destination_bit = destination_start + copied;
            let destination_offset = destination_bit % 8;
            let take = core::cmp::min(source.len() - copied, 8 - destination_offset);
            let value = load_be_chunk(source.bytes, source.offset + copied, take);
            let shift = 8 - destination_offset - take;
            let mask = (((1u16 << take) - 1) as u8) << shift;
            let slot = &mut self.bytes[destination_bit / 8];
            *slot = (*slot & !mask) | ((value << shift) & mask);
            copied += take;
        }
    }

    /// Return an immutable view after borrowing the mutable view for this call.
    #[inline]
    pub fn as_bitslice(&self) -> BitSlice<'_, T, O> {
        BitSlice::from_parts(self.bytes, self.offset, self.len)
    }
}

/// A packed, growable bit vector.
///
/// Storage is byte-based regardless of `T`. `O` controls byte conversion in
/// [`from_slice`](Self::from_slice) and [`into_vec`](Self::into_vec).
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct BitVec<T = u8, O = Msb0> {
    bytes: Vec<u8>,
    len: usize,
    _marker: PhantomData<(T, O)>,
}

#[cfg(feature = "alloc")]
impl<T, O> BitVec<T, O> {
    /// Create an empty bit vector.
    #[inline]
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            len: 0,
            _marker: PhantomData,
        }
    }

    /// Create a bit vector containing `len` copies of `value`.
    #[inline]
    pub fn repeat(value: bool, len: usize) -> Self {
        let mut result = Self {
            bytes: alloc::vec![if value { 0xff } else { 0 }; len.div_ceil(8)],
            len,
            _marker: PhantomData,
        };
        result.clear_padding_bits();
        result
    }

    /// Create a bit vector from a byte-aligned bit sequence.
    ///
    /// # Panics
    /// Panics if `bytes.len() * 8` cannot be represented by `usize`.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Self
    where
        O: BitOrder,
    {
        let len = bit_len_from_bytes(bytes.len());
        let mut bytes = bytes.to_vec();
        O::decode(&mut bytes);
        Self {
            bytes,
            len,
            _marker: PhantomData,
        }
    }

    /// Create a bit vector from boolean bits.
    #[inline]
    pub fn from_bits(bits: &[bool]) -> Self {
        let mut result = Self {
            bytes: alloc::vec![0; bits.len().div_ceil(8)],
            len: bits.len(),
            _marker: PhantomData,
        };
        for (slot, chunk) in result.bytes.iter_mut().zip(bits.chunks(8)) {
            let mut value = 0u8;
            for &bit in chunk {
                value = (value << 1) | u8::from(bit);
            }
            *slot = value << (8 - chunk.len());
        }
        result
    }

    /// Return the number of bits in this vector.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether this vector contains no bits.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate over the stored bits.
    #[inline]
    pub fn iter(&self) -> BitIter<'_, T, O> {
        self.as_bitslice().iter()
    }

    /// Return an immutable view over this vector.
    #[inline]
    pub fn as_bitslice(&self) -> BitSlice<'_, T, O> {
        BitSlice::from_parts(&self.bytes, 0, self.len)
    }

    /// Return a mutable view over this vector.
    #[inline]
    pub fn as_mut_bitslice(&mut self) -> BitSliceMut<'_, T, O> {
        BitSliceMut::from_parts(&mut self.bytes, 0, self.len)
    }

    /// Borrow the packed bytes for crate-internal I/O fast paths.
    #[inline]
    pub(crate) fn as_raw_slice_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Return the bit at `index`.
    #[inline]
    pub fn get(&self, index: usize) -> bool {
        self.as_bitslice().get(index)
    }

    /// Append one bit.
    ///
    /// # Panics
    /// Panics if the resulting bit length cannot be represented by `usize`.
    #[inline]
    pub fn push(&mut self, value: bool) {
        let next_len = next_bit_len(self.len);
        if self.len.is_multiple_of(8) {
            self.bytes.push(0);
        }
        set_bit(&mut self.bytes, self.len, value);
        self.len = next_len;
    }

    /// Append all bits in `source`.
    #[inline]
    pub fn extend_from_bitslice<U, P>(&mut self, source: &BitSlice<'_, U, P>) {
        for bit in source.iter() {
            self.push(bit);
        }
    }

    /// Return a packed byte representation, padding the last byte with zeroes.
    #[inline]
    pub fn into_vec(mut self) -> Vec<u8>
    where
        O: BitOrder,
    {
        self.clear_padding_bits();
        O::encode(&mut self.bytes);
        self.bytes
    }

    /// Return this vector unchanged. This mirrors the old macro helper.
    #[inline]
    pub fn to_bitvec(self) -> Self {
        self
    }

    /// Load this bit vector as a big-endian integer.
    #[inline]
    pub fn load_be<V: BitValue>(&self) -> V {
        self.as_bitslice().load_be()
    }

    /// Load this bit vector as a little-endian integer.
    #[inline]
    pub fn load_le<V: BitValue>(&self) -> V {
        self.as_bitslice().load_le()
    }

    /// Load this bit vector using its packed MSB-first representation.
    #[inline]
    pub fn load<V: BitValue>(&self) -> V {
        self.as_bitslice().load()
    }

    fn clear_padding_bits(&mut self) {
        if let Some(last) = self.bytes.last_mut() {
            let used = self.len % 8;
            if used != 0 {
                *last &= 0xff << (8 - used);
            }
        }
    }
}

#[cfg(feature = "alloc")]
impl<T, O> core::iter::FromIterator<bool> for BitVec<T, O> {
    fn from_iter<I: IntoIterator<Item = bool>>(iter: I) -> Self {
        let mut result = Self::new();
        for bit in iter {
            result.push(bit);
        }
        result
    }
}

#[cfg(feature = "alloc")]
impl<T, O> Default for BitVec<T, O> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl<T, O, U, P> PartialEq<BitVec<U, P>> for BitVec<T, O> {
    fn eq(&self, other: &BitVec<U, P>) -> bool {
        self.as_bitslice() == other.as_bitslice()
    }
}

#[cfg(feature = "alloc")]
impl<T, O> Eq for BitVec<T, O> {}

#[cfg(feature = "alloc")]
impl<T, O> BitSource for BitVec<T, O> {
    #[inline]
    fn bit_slice(&self) -> BitSlice<'_, u8, Msb0> {
        BitSlice::from_parts(&self.bytes, 0, self.len)
    }
}

#[cfg(feature = "alloc")]
impl<T, O> fmt::Debug for BitVec<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BitVec")
            .field("len", &self.len)
            .field("bytes", &self.bytes)
            .finish()
    }
}

/// Integer types supported by packed bit loading.
///
/// Implementations must set `BITS` to a value no greater than 128. Loading
/// asserts this limit because packed values are accumulated in `u128`.
pub trait BitValue: Copy {
    /// Number of bits in the integer type.
    const BITS: usize;

    /// Convert a non-negative packed value into this type.
    fn from_u128(value: u128) -> Self;
}

macro_rules! impl_bit_value {
    ($($type:ty),* $(,)?) => {
        $(
            impl BitValue for $type {
                const BITS: usize = <$type>::BITS as usize;

                #[inline]
                fn from_u128(value: u128) -> Self {
                    value as $type
                }
            }
        )*
    };
}

impl_bit_value!(u8, u16, u32, u64, u128, usize);

#[inline]
fn assert_bit_value_len<V: BitValue>(len: usize) {
    assert!(V::BITS <= u128::BITS as usize, "BitValue::BITS exceeds 128");
    assert!(len <= V::BITS);
}

#[inline]
fn bit_len_from_bytes(len: usize) -> usize {
    len.checked_mul(u8::BITS as usize)
        .expect("bit length exceeds usize::MAX")
}

#[inline]
fn load_be_chunk(bytes: &[u8], offset: usize, len: usize) -> u8 {
    debug_assert!((1..=8).contains(&len));
    let byte_index = offset / 8;
    let within_byte = offset % 8;
    let mut pair = u16::from(bytes[byte_index]) << 8;
    if within_byte + len > 8 {
        pair |= u16::from(bytes[byte_index + 1]);
    }
    let shift = 16 - within_byte - len;
    let mask = (1u16 << len) - 1;
    ((pair >> shift) & mask) as u8
}

#[inline(never)]
fn load_be_unaligned(bytes: &[u8], offset: usize, len: usize) -> u128 {
    if len == 0 {
        return 0;
    }
    let within_byte = offset % 8;
    let byte_start = offset / 8;
    let window_bits = within_byte + len;
    let byte_len = window_bits.div_ceil(8);
    let mut value = if byte_len <= 16 {
        load_be_bytes(bytes, byte_start, byte_len) >> (byte_len * 8 - window_bits)
    } else {
        let first_len = 8 - within_byte;
        let first = load_be_chunk(bytes, offset, first_len);
        let rest_len = len - first_len;
        let rest_byte_start = (offset + first_len) / 8;
        let rest_byte_len = rest_len.div_ceil(8);
        let rest =
            load_be_bytes(bytes, rest_byte_start, rest_byte_len) >> (rest_byte_len * 8 - rest_len);
        (u128::from(first) << rest_len) | (rest & low_mask(rest_len))
    };
    value &= low_mask(len);
    value
}

#[inline(never)]
fn load_le_unaligned(bytes: &[u8], offset: usize, len: usize) -> u128 {
    // `load_le` reverses the order of byte-sized groups while keeping each
    // group's bits in MSB-first order. Keep the partial edge groups and reverse
    // the complete interior bytes as one integer operation.
    let leading_len = if offset.is_multiple_of(8) {
        0
    } else {
        core::cmp::min(len, 8 - offset % 8)
    };
    let trailing_len = if leading_len == len {
        0
    } else {
        (offset + len) % 8
    };
    let middle_len = len - leading_len - trailing_len;
    debug_assert_eq!(middle_len % 8, 0);

    let leading = if leading_len == 0 {
        0
    } else {
        u128::from(load_be_chunk(bytes, offset, leading_len))
    };
    let middle = load_be_bytes(bytes, (offset + leading_len) / 8, middle_len / 8);
    let middle_reversed = if middle_len == 0 {
        0
    } else {
        (middle << (u128::BITS as usize - middle_len)).swap_bytes() & low_mask(middle_len)
    };
    let trailing = if trailing_len == 0 {
        0
    } else {
        u128::from(load_be_chunk(
            bytes,
            offset + len - trailing_len,
            trailing_len,
        ))
    };

    (trailing << (middle_len + leading_len)) | (middle_reversed << leading_len) | leading
}

#[inline]
fn load_be_bytes(bytes: &[u8], start: usize, len: usize) -> u128 {
    let bytes = &bytes[start..start + len];
    match bytes.len() {
        0 => 0,
        1 => u128::from(bytes[0]),
        2 => u128::from(u16::from_be_bytes(bytes.try_into().unwrap())),
        4 => u128::from(u32::from_be_bytes(bytes.try_into().unwrap())),
        8 => u128::from(u64::from_be_bytes(bytes.try_into().unwrap())),
        16 => u128::from_be_bytes(bytes.try_into().unwrap()),
        _ => bytes
            .iter()
            .fold(0, |value, &byte| (value << 8) | u128::from(byte)),
    }
}

#[inline]
fn low_mask(width: usize) -> u128 {
    if width == u128::BITS as usize {
        u128::MAX
    } else {
        (1u128 << width) - 1
    }
}

#[inline]
#[cfg(feature = "alloc")]
fn next_bit_len(len: usize) -> usize {
    len.checked_add(1)
        .expect("bit vector length exceeds usize::MAX")
}

#[inline]
fn chunk_end(index: usize, len: usize, chunk_len: usize) -> usize {
    index + core::cmp::min(chunk_len, len - index)
}

#[inline]
fn bit_at(bytes: &[u8], index: usize) -> bool {
    bytes[index / 8] & (0x80 >> (index % 8)) != 0
}

#[inline]
fn set_bit(bytes: &mut [u8], index: usize, value: bool) {
    let mask = 0x80 >> (index % 8);
    if value {
        bytes[index / 8] |= mask;
    } else {
        bytes[index / 8] &= !mask;
    }
}

/// Return the highest index of the requested bit when each byte is traversed LSB-first.
#[inline]
fn last_lsb_matching(bytes: &[u8], set: bool) -> Option<usize> {
    bytes
        .iter()
        .enumerate()
        .rev()
        .find_map(|(byte_index, byte)| {
            let byte = if set { *byte } else { !*byte };
            if byte == 0 {
                None
            } else {
                Some(byte_index * 8 + (u8::BITS as usize - 1 - byte.leading_zeros() as usize))
            }
        })
}

/// Return the highest set-bit index when each byte is traversed LSB-first.
pub(crate) fn last_one_lsb(bytes: &[u8]) -> Option<usize> {
    last_lsb_matching(bytes, true)
}

/// Return the highest clear-bit index when each byte is traversed LSB-first.
pub(crate) fn last_zero_lsb(bytes: &[u8]) -> Option<usize> {
    last_lsb_matching(bytes, false)
}

#[doc(hidden)]
#[macro_export]
macro_rules! __deku_bit {
    (0) => {
        false
    };
    (1) => {
        true
    };
    (false) => {
        false
    };
    (true) => {
        true
    };
}

#[cfg(feature = "alloc")]
#[doc(hidden)]
#[macro_export]
macro_rules! bitvec {
    ($store:ty, $order:ty; $($bit:tt),* $(,)?) => {
        $crate::bitvec::BitVec::<$store, $order>::from_bits(&[$($crate::__deku_bit!($bit)),*])
    };
    ($($bit:tt),* $(,)?) => {
        $crate::bitvec::BitVec::<u8, $crate::bitvec::Msb0>::from_bits(
            &[$($crate::__deku_bit!($bit)),*]
        )
    };
}

#[cfg(feature = "alloc")]
#[doc(hidden)]
#[macro_export]
macro_rules! bitarr {
    ($store:ty, $order:ty; $bit:tt; $len:expr $(,)?) => {
        $crate::bitvec::BitVec::<$store, $order>::repeat($crate::__deku_bit!($bit), $len)
    };
}

#[cfg(feature = "alloc")]
#[doc(hidden)]
#[macro_export]
macro_rules! bits {
    ($store:ty, $order:ty; $($bit:tt),* $(,)?) => {
        $crate::bitvec::BitVec::<$store, $order>::from_bits(&[$($crate::__deku_bit!($bit)),*])
    };
    ($($bit:tt),* $(,)?) => {
        $crate::bitvec::BitVec::<u8, $crate::bitvec::Msb0>::from_bits(
            &[$($crate::__deku_bit!($bit)),*]
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_bits() -> [bool; 136] {
        core::array::from_fn(|index| (index * 73 + 19) % 11 < 5)
    }

    fn pack_reference(bits: &[bool; 136]) -> [u8; 17] {
        let mut bytes = [0; 17];
        for (index, &bit) in bits.iter().enumerate() {
            if bit {
                bytes[index / 8] |= 0x80 >> (index % 8);
            }
        }
        bytes
    }

    fn load_be_reference(bits: &[bool]) -> u128 {
        bits.iter()
            .fold(0u128, |value, &bit| (value << 1) | u128::from(bit))
    }

    fn load_le_reference(bits: &[bool], offset: usize) -> u128 {
        let mut value = 0u128;
        let mut end = offset + bits.len();
        while end > offset {
            let start = core::cmp::max(offset, (end - 1) / 8 * 8);
            let group = &bits[start - offset..end - offset];
            value = (value << group.len()) | load_be_reference(group);
            end = start;
        }
        value
    }

    #[derive(Clone, Copy)]
    struct TooWide;

    impl BitValue for TooWide {
        const BITS: usize = 129;

        fn from_u128(_: u128) -> Self {
            Self
        }
    }

    #[test]
    fn chunks_reach_the_end_without_overflowing() {
        assert_eq!(
            chunk_end(usize::MAX / 2 + 1, usize::MAX, usize::MAX / 2 + 1),
            usize::MAX
        );
    }

    #[test]
    fn chunks_include_a_partial_final_chunk() {
        let bytes = [0b1011_0010, 0b0110_0000];
        let mut chunks = BitSlice::<u8, Msb0>::from_slice(&bytes).chunks(6);

        assert_eq!(chunks.next().unwrap().load_be::<u8>(), 0b10_1100);
        assert_eq!(chunks.next().unwrap().load_be::<u8>(), 0b10_0110);
        assert_eq!(chunks.next().unwrap().load_be::<u8>(), 0);
        assert!(chunks.next().is_none());
    }

    #[test]
    fn slices_match_reference_for_all_offsets_and_lengths() {
        let expected = reference_bits();
        let bytes = pack_reference(&expected);
        let all_bits = BitSlice::<u8, Msb0>::from_slice(&bytes);

        for offset in 0..8 {
            for len in 0..=128 {
                let end = offset + len;
                let expected_slice = &expected[offset..end];
                let slice = all_bits.subslice(offset, end);

                assert!(slice.iter().eq(expected_slice.iter().copied()));
                assert_eq!(slice.load_be::<u128>(), load_be_reference(expected_slice));
                assert_eq!(
                    slice.load_le::<u128>(),
                    load_le_reference(expected_slice, offset)
                );
            }
        }

        let mut mutable_bytes = [0; 17];
        let mut mutable = BitSliceMut::<u8, Msb0>::from_slice(&mut mutable_bytes);
        for (index, &bit) in expected.iter().enumerate() {
            mutable.set(index, bit);
        }
        assert!(mutable.as_bitslice().iter().eq(expected));
        assert_eq!(mutable_bytes, bytes);
    }

    #[test]
    fn chunks_match_reference_across_offsets_and_chunk_lengths() {
        let expected = reference_bits();
        let bytes = pack_reference(&expected);
        let all_bits = BitSlice::<u8, Msb0>::from_slice(&bytes);

        for offset in 0..8 {
            let end = offset + 128;
            let expected_slice = &expected[offset..end];
            let slice = all_bits.subslice(offset, end);
            for chunk_len in 1..=16 {
                assert!(
                    slice
                        .chunks(chunk_len)
                        .flat_map(|chunk| chunk.iter())
                        .eq(expected_slice.iter().copied())
                );
            }
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn vectors_match_reference_for_all_lengths_and_orders() {
        let expected = reference_bits();

        for len in 0..=expected.len() {
            let bits = &expected[..len];
            let vector = BitVec::<u8, Msb0>::from_bits(bits);
            assert!(vector.iter().eq(bits.iter().copied()));

            let mut pushed = BitVec::<u8, Msb0>::new();
            for &bit in bits {
                pushed.push(bit);
            }
            assert_eq!(pushed, vector);

            let mut packed = alloc::vec![0; len.div_ceil(8)];
            for (index, &bit) in bits.iter().enumerate() {
                if bit {
                    packed[index / 8] |= 0x80 >> (index % 8);
                }
            }
            assert_eq!(vector.into_vec(), packed);

            let mut lsb0_packed = packed.clone();
            for byte in &mut lsb0_packed {
                *byte = byte.reverse_bits();
            }
            let lsb0 = BitVec::<u8, Lsb0>::from_bits(bits);
            assert_eq!(lsb0.into_vec(), lsb0_packed);

            let decoded = BitVec::<u8, Lsb0>::from_slice(&lsb0_packed);
            assert!(decoded.iter().take(len).eq(bits.iter().copied()));
            assert!(decoded.iter().skip(len).all(|bit| !bit));
        }
    }

    #[test]
    #[should_panic(expected = "BitValue::BITS exceeds 128")]
    fn load_be_rejects_values_wider_than_its_accumulator() {
        let _ = BitSlice::<u8, Msb0>::empty().load_be::<TooWide>();
    }

    #[test]
    #[should_panic(expected = "BitValue::BITS exceeds 128")]
    fn load_le_rejects_values_wider_than_its_accumulator() {
        let _ = BitSlice::<u8, Msb0>::empty().load_le::<TooWide>();
    }

    #[test]
    #[should_panic(expected = "bit length exceeds usize::MAX")]
    fn byte_lengths_that_overflow_the_bit_count_are_rejected() {
        let _ = bit_len_from_bytes(usize::MAX / 8 + 1);
    }

    #[test]
    #[cfg(feature = "alloc")]
    #[should_panic(expected = "bit vector length exceeds usize::MAX")]
    fn bit_vector_length_cannot_overflow() {
        let _ = next_bit_len(usize::MAX);
    }
}
