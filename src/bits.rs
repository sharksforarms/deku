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
/// arbitrary bit offsets without allocating.
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
    #[inline]
    pub fn from_slice(bytes: &'a [u8]) -> Self {
        Self::from_parts(bytes, 0, bytes.len().saturating_mul(8))
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
        assert!(offset.checked_add(len).is_some());
        assert!(offset.saturating_add(len) <= bytes.len().saturating_mul(8));
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
    #[inline]
    pub fn load_be<V: BitValue>(&self) -> V {
        assert!(self.len <= V::BITS);
        if let Some(bytes) = self.aligned_bytes() {
            let mut value = 0u128;
            for &byte in bytes {
                value = (value << 8) | u128::from(byte);
            }
            return V::from_u128(value);
        }
        let mut value = 0u128;
        for bit in self.iter() {
            value = (value << 1) | u128::from(bit);
        }
        V::from_u128(value)
    }

    /// Load this bit sequence as a little-endian integer.
    #[inline]
    pub fn load_le<V: BitValue>(&self) -> V {
        assert!(self.len <= V::BITS);
        if let Some(bytes) = self.aligned_bytes() {
            let mut value = 0u128;
            for &byte in bytes.iter().rev() {
                value = (value << 8) | u128::from(byte);
            }
            return V::from_u128(value);
        }
        let mut value = 0u128;
        let mut end = self.offset + self.len;

        // The storage element order is little-endian, while the bits inside
        // each byte retain their packed MSB-first representation. This is the
        // behavior callers get from bitvec's `BitSlice<u8, Msb0>::load_le`:
        // byte-sized pieces are concatenated from the last piece to the first.
        while end > self.offset {
            let byte_start = (end - 1) / 8 * 8;
            let start = core::cmp::max(self.offset, byte_start);
            let chunk = self.subslice(start - self.offset, end - self.offset);
            value = (value << chunk.len()) | u128::from(chunk.load_be::<u8>());
            end = start;
        }
        V::from_u128(value)
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
        let end = core::cmp::min(self.index + self.chunk_len, self.slice.len());
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
    #[inline]
    pub fn from_slice(bytes: &'a mut [u8]) -> Self {
        let len = bytes.len().saturating_mul(8);
        Self::from_parts(bytes, 0, len)
    }

    /// Create a mutable view over a bit range in `bytes`.
    #[inline]
    pub(crate) fn from_parts(bytes: &'a mut [u8], offset: usize, len: usize) -> Self {
        assert!(offset.checked_add(len).is_some());
        assert!(offset.saturating_add(len) <= bytes.len().saturating_mul(8));
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
        for (index, bit) in source.iter().enumerate() {
            self.set(start + index, bit);
        }
    }

    /// Return an immutable view after borrowing the mutable view for this call.
    #[inline]
    pub fn as_bitslice(&self) -> BitSlice<'_, T, O> {
        BitSlice::from_parts(self.bytes, self.offset, self.len)
    }
}

/// A packed, growable bit vector.
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
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Self
    where
        O: BitOrder,
    {
        let mut bytes = bytes.to_vec();
        O::decode(&mut bytes);
        let len = bytes.len().saturating_mul(8);
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
    #[inline]
    pub fn push(&mut self, value: bool) {
        if self.len.is_multiple_of(8) {
            self.bytes.push(0);
        }
        set_bit(&mut self.bytes, self.len, value);
        self.len += 1;
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

#[doc(hidden)]
#[macro_export]
macro_rules! bitarr {
    ($store:ty, $order:ty; $bit:tt; $len:expr $(,)?) => {
        $crate::bitvec::BitVec::<$store, $order>::repeat($crate::__deku_bit!($bit), $len)
    };
}

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
