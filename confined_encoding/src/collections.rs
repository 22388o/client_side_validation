// LNP/BP client-side-validation foundation libraries implementing LNPBP
// specifications & standards (LNPBP-4, 7, 8, 9, 42, 81)
//
// Written in 2019-2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the Apache 2.0 License along with this
// software. If not, see <https://opensource.org/licenses/Apache-2.0>.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::io;
use std::io::{Read, Write};
use std::ops::{Range, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive};

use crate::{ConfinedDecode, ConfinedEncode, Error};

/// In terms of strict encoding, ranges are encoded as a tuples of two values:
/// start and end.
impl<T> ConfinedEncode for Range<T>
where
    T: ConfinedEncode,
{
    fn confined_encode<E: Write>(&self, mut e: E) -> Result<usize, Error> {
        Ok(self.start.confined_encode(&mut e)?
            + self.end.confined_encode(&mut e)?)
    }
}

/// In terms of strict decoding, ranges are represented as a tuples of two
/// values: start and end.
impl<T> ConfinedDecode for Range<T>
where
    T: ConfinedDecode,
{
    fn confined_decode<D: Read>(mut d: D) -> Result<Self, Error> {
        Ok(Range {
            start: T::confined_decode(&mut d)?,
            end: T::confined_decode(&mut d)?,
        })
    }
}

/// In terms of strict encoding, inclusive ranges are encoded as a tuples of two
/// values: start and end.
impl<T> ConfinedEncode for RangeInclusive<T>
where
    T: ConfinedEncode,
{
    fn confined_encode<E: Write>(&self, mut e: E) -> Result<usize, Error> {
        Ok(self.start().confined_encode(&mut e)?
            + self.end().confined_encode(&mut e)?)
    }
}

/// In terms of strict decoding, inclusive ranges are represented as a tuples of
/// two values: start and end.
impl<T> ConfinedDecode for RangeInclusive<T>
where
    T: ConfinedDecode,
{
    fn confined_decode<D: Read>(mut d: D) -> Result<Self, Error> {
        Ok(RangeInclusive::new(
            T::confined_decode(&mut d)?,
            T::confined_decode(&mut d)?,
        ))
    }
}

/// In terms of strict encoding, partial ranges are encoded as a single value.
impl<T> ConfinedEncode for RangeFrom<T>
where
    T: ConfinedEncode,
{
    #[inline]
    fn confined_encode<E: Write>(&self, mut e: E) -> Result<usize, Error> {
        self.start.confined_encode(&mut e)
    }
}

/// In terms of strict decoding, partial ranges are represented as a single
/// value.
impl<T> ConfinedDecode for RangeFrom<T>
where
    T: ConfinedDecode,
{
    #[inline]
    fn confined_decode<D: Read>(mut d: D) -> Result<Self, Error> {
        Ok(RangeFrom {
            start: T::confined_decode(&mut d)?,
        })
    }
}

/// In terms of strict encoding, partial ranges are encoded as a single value.
impl<T> ConfinedEncode for RangeTo<T>
where
    T: ConfinedEncode,
{
    #[inline]
    fn confined_encode<E: Write>(&self, mut e: E) -> Result<usize, Error> {
        self.end.confined_encode(&mut e)
    }
}

/// In terms of strict decoding, partial ranges are represented as a single
/// value.
impl<T> ConfinedDecode for RangeTo<T>
where
    T: ConfinedDecode,
{
    #[inline]
    fn confined_decode<D: Read>(mut d: D) -> Result<Self, Error> {
        Ok(RangeTo {
            end: T::confined_decode(&mut d)?,
        })
    }
}

/// In terms of strict encoding, partial ranges are encoded as a single value.
impl<T> ConfinedEncode for RangeToInclusive<T>
where
    T: ConfinedEncode,
{
    #[inline]
    fn confined_encode<E: Write>(&self, mut e: E) -> Result<usize, Error> {
        self.end.confined_encode(&mut e)
    }
}

/// In terms of strict decoding, partial ranges are represented as a single
/// value.
impl<T> ConfinedDecode for RangeToInclusive<T>
where
    T: ConfinedDecode,
{
    #[inline]
    fn confined_decode<D: Read>(mut d: D) -> Result<Self, Error> {
        Ok(RangeToInclusive {
            end: T::confined_decode(&mut d)?,
        })
    }
}

/// In terms of strict encoding, `Option` (optional values) are  
/// represented by a *significator byte*, which MUST be either `0` (for no
/// value present) or `1`, followed by the value strict encoding.
impl<T> ConfinedEncode for Option<T>
where
    T: ConfinedEncode,
{
    fn confined_encode<E: io::Write>(&self, mut e: E) -> Result<usize, Error> {
        Ok(match self {
            None => confined_encode_list!(e; 0u8),
            Some(val) => confined_encode_list!(e; 1u8, val),
        })
    }
}

/// In terms of strict encoding, `Option` (optional values) are  
/// represented by a *significator byte*, which MUST be either `0` (for no
/// value present) or `1`, followed by the value strict encoding.
/// For decoding an attempt to read `Option` from a encoded non-0
/// or non-1 length Vec will result in `Error::WrongOptionalEncoding`.
impl<T> ConfinedDecode for Option<T>
where
    T: ConfinedDecode,
{
    fn confined_decode<D: io::Read>(mut d: D) -> Result<Self, Error> {
        let len = u8::confined_decode(&mut d)?;
        match len {
            0 => Ok(None),
            1 => Ok(Some(T::confined_decode(&mut d)?)),
            invalid => Err(Error::WrongOptionalEncoding(invalid)),
        }
    }
}

/// In terms of strict encoding, a slice is stored in form of
/// usize-encoded length (see `ConfinedEncode` implementation for `usize`
/// type for encoding platform-independent constant-length
/// encoding rules) followed by a consequently-encoded vec items,
/// according to their type.
impl<T> ConfinedEncode for [T]
where
    T: ConfinedEncode,
{
    fn confined_encode<E: io::Write>(&self, mut e: E) -> Result<usize, Error> {
        let len = self.len() as usize;
        // We handle oversize problems at the level of `usize` value
        // serializaton
        let mut encoded = len.confined_encode(&mut e)?;
        for item in self {
            encoded += item.confined_encode(&mut e)?;
        }
        Ok(encoded)
    }
}

/// In terms of strict encoding, `Vec` is stored in form of
/// usize-encoded length (see `ConfinedEncode` implementation for `usize`
/// type for encoding platform-independent constant-length
/// encoding rules) followed by a consequently-encoded vec items,
/// according to their type.
impl<T> ConfinedEncode for Vec<T>
where
    T: ConfinedEncode,
{
    fn confined_encode<E: io::Write>(&self, e: E) -> Result<usize, Error> {
        self.as_slice().confined_encode(e)
    }
}

/// In terms of strict encoding, `Vec` is stored in form of
/// usize-encoded length (see `ConfinedEncode` implementation for `usize`
/// type for encoding platform-independent constant-length
/// encoding rules) followed by a consequently-encoded vec items,
/// according to their type.
///
/// An attempt to encode `Vec` with more items than can fit in `usize`
/// encoding rules will result in `Error::ExceedMaxItems`.
impl<T> ConfinedDecode for Vec<T>
where
    T: ConfinedDecode,
{
    fn confined_decode<D: io::Read>(mut d: D) -> Result<Self, Error> {
        let len = usize::confined_decode(&mut d)?;
        let mut data = Vec::<T>::with_capacity(len as usize);
        for _ in 0..len {
            data.push(T::confined_decode(&mut d)?);
        }
        Ok(data)
    }
}

/// Strict encoding for a unique value collection represented by a rust
/// `BTreeSet` type is performed in the same way as `Vec` encoding.
/// NB: Array members must are ordered with the sort operation, so type
/// `T` must implement `Ord` trait in such a way that it produces
/// deterministically-sorted result
impl<T> ConfinedEncode for BTreeSet<T>
where
    T: ConfinedEncode + Eq + Ord + Debug,
{
    fn confined_encode<E: io::Write>(&self, mut e: E) -> Result<usize, Error> {
        let len = self.len() as usize;
        let mut encoded = len.confined_encode(&mut e)?;
        let mut vec: Vec<&T> = self.iter().collect();
        vec.sort();
        for item in vec {
            encoded += item.confined_encode(&mut e)?;
        }
        Ok(encoded)
    }
}

/// Strict decoding of a unique value collection represented by a rust
/// `BTreeSet` type is performed alike `Vec` decoding with the only
/// exception: if the repeated value met a [Error::RepeatedValue] is
/// returned.
impl<T> ConfinedDecode for BTreeSet<T>
where
    T: ConfinedDecode + Eq + Ord + Debug,
{
    fn confined_decode<D: io::Read>(mut d: D) -> Result<Self, Error> {
        let len = usize::confined_decode(&mut d)?;
        let mut data = BTreeSet::<T>::new();
        for _ in 0..len {
            let val = T::confined_decode(&mut d)?;
            if let Some(max) = data.iter().max() {
                if max > &val {
                    // TODO: Introduce new error type on 2.0 release
                    return Err(Error::DataIntegrityError(format!(
                        "encoded values are not deterministically ordered: \
                         value `{:?}` should go before `{:?}`",
                        val, max
                    )));
                }
            }
            if data.contains(&val) {
                return Err(Error::RepeatedValue(format!("{:?}", val)));
            }
            data.insert(val);
        }
        Ok(data)
    }
}

/// LNP/BP library uses `BTreeMap<usize, T: ConfinedEncode>`s to encode
/// ordered lists, where the position of the list item must be fixed, since
/// the item is referenced from elsewhere by its index. Thus, the library
/// does not supports and recommends not to support strict encoding
/// of any other `BTreeMap` variants.
///
/// Strict encoding of the `BTreeMap<usize, T>` type is performed
/// by converting into a fixed-order `Vec<T>` and serializing it according
/// to the `Vec` strict encoding rules.
impl<K, V> ConfinedEncode for BTreeMap<K, V>
where
    K: ConfinedEncode + Ord + Clone,
    V: ConfinedEncode + Clone,
{
    fn confined_encode<E: io::Write>(&self, mut e: E) -> Result<usize, Error> {
        let len = self.len() as usize;
        let encoded = len.confined_encode(&mut e)?;

        self.iter().try_fold(encoded, |mut acc, (key, val)| {
            acc += key.confined_encode(&mut e)?;
            acc += val.confined_encode(&mut e)?;
            Ok(acc)
        })
    }
}

/// LNP/BP library uses `BTreeMap<usize, T: ConfinedEncode>`s to encode
/// ordered lists, where the position of the list item must be fixed, since
/// the item is referenced from elsewhere by its index. Thus, the library
/// does not supports and recommends not to support strict encoding
/// of any other `BTreeMap` variants.
///
/// Strict encoding of the `BTreeMap<usize, T>` type is performed
/// by converting into a fixed-order `Vec<T>` and serializing it according
/// to the `Vec` strict encoding rules.
impl<K, V> ConfinedDecode for BTreeMap<K, V>
where
    K: ConfinedDecode + Ord + Clone + Debug,
    V: ConfinedDecode + Clone,
{
    fn confined_decode<D: io::Read>(mut d: D) -> Result<Self, Error> {
        let len = usize::confined_decode(&mut d)?;
        let mut map = BTreeMap::<K, V>::new();
        for _ in 0..len {
            let key = K::confined_decode(&mut d)?;
            let val = V::confined_decode(&mut d)?;
            if let Some(max) = map.keys().max() {
                if max > &key {
                    // TODO: Introduce new error type on 2.0 release
                    return Err(Error::DataIntegrityError(format!(
                        "encoded values are not deterministically ordered: \
                         value `{:?}` should go before `{:?}`",
                        key, max
                    )));
                }
            }
            if map.contains_key(&key) {
                return Err(Error::RepeatedValue(format!("{:?}", key)));
            }
            map.insert(key, val);
        }
        Ok(map)
    }
}

/// Two-component tuples are encoded as they were fields in the parent
/// data structure
impl<K, V> ConfinedEncode for (K, V)
where
    K: ConfinedEncode + Clone,
    V: ConfinedEncode + Clone,
{
    fn confined_encode<E: io::Write>(&self, mut e: E) -> Result<usize, Error> {
        Ok(self.0.confined_encode(&mut e)? + self.1.confined_encode(&mut e)?)
    }
}

/// Two-component tuples are decoded as they were fields in the parent
/// data structure
impl<K, V> ConfinedDecode for (K, V)
where
    K: ConfinedDecode + Clone,
    V: ConfinedDecode + Clone,
{
    fn confined_decode<D: io::Read>(mut d: D) -> Result<Self, Error> {
        let a = K::confined_decode(&mut d)?;
        let b = V::confined_decode(&mut d)?;
        Ok((a, b))
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::confined_serialize;

    /// Test for checking the following rule from LNPBP-5:
    ///
    /// `Option<T>` of any type T, which are set to `Option::None` value MUST
    /// encode as two zero bytes and it MUST be possible to decode optional
    /// of any type from two zero bytes which MUST result in `Option::None`
    /// value.
    #[test]
    fn test_option_encode_none() {
        let o1: Option<u8> = None;
        let o2: Option<u64> = None;

        let two_zero_bytes = &vec![0u8][..];

        assert_eq!(confined_serialize(&o1).unwrap(), two_zero_bytes);
        assert_eq!(confined_serialize(&o2).unwrap(), two_zero_bytes);

        assert_eq!(
            Option::<u8>::confined_decode(two_zero_bytes).unwrap(),
            None
        );
        assert_eq!(
            Option::<u64>::confined_decode(two_zero_bytes).unwrap(),
            None
        );
    }

    /// Test for checking the following rule from LNPBP-5:
    ///
    /// `Option<T>` of any type T, which are set to `Option::Some<T>` value MUST
    /// encode as a `Vec<T>` structure containing a single item equal to the
    /// `Option::unwrap()` value.
    #[test]
    fn test_option_encode_some() {
        let o1: Option<u8> = Some(0);
        let o2: Option<u8> = Some(13);
        let o3: Option<u8> = Some(0xFF);
        let o4: Option<u64> = Some(13);
        let o5: Option<u64> = Some(0x1FF);
        let o6: Option<u64> = Some(0xFFFFFFFFFFFFFFFF);
        let o7: Option<usize> = Some(13);
        let o8: Option<usize> = Some(0xFFFFFFFFFFFFFFFF);

        let byte_0 = &[1u8, 0u8][..];
        let byte_13 = &[1u8, 13u8][..];
        let byte_255 = &[1u8, 0xFFu8][..];
        let word_13 = &[1u8, 13u8, 0u8][..];
        let qword_13 = &[1u8, 13u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8][..];
        let qword_256 =
            &[1u8, 0xFFu8, 0x01u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8][..];
        let qword_max = &[
            1u8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8,
        ][..];

        assert_eq!(confined_serialize(&o1).unwrap(), byte_0);
        assert_eq!(confined_serialize(&o2).unwrap(), byte_13);
        assert_eq!(confined_serialize(&o3).unwrap(), byte_255);
        assert_eq!(confined_serialize(&o4).unwrap(), qword_13);
        assert_eq!(confined_serialize(&o5).unwrap(), qword_256);
        assert_eq!(confined_serialize(&o6).unwrap(), qword_max);
        assert_eq!(confined_serialize(&o7).unwrap(), word_13);
        assert!(confined_serialize(&o8).err().is_some());

        assert_eq!(Option::<u8>::confined_decode(byte_0).unwrap(), Some(0));
        assert_eq!(Option::<u8>::confined_decode(byte_13).unwrap(), Some(13));
        assert_eq!(
            Option::<u8>::confined_decode(byte_255).unwrap(),
            Some(0xFF)
        );
        assert_eq!(Option::<u64>::confined_decode(qword_13).unwrap(), Some(13));
        assert_eq!(
            Option::<u64>::confined_decode(qword_256).unwrap(),
            Some(0x1FF)
        );
        assert_eq!(
            Option::<u64>::confined_decode(qword_max).unwrap(),
            Some(0xFFFFFFFFFFFFFFFF)
        );
        assert_eq!(
            Option::<usize>::confined_decode(word_13).unwrap(),
            Some(13)
        );
        assert_eq!(
            Option::<usize>::confined_decode(qword_max).unwrap(),
            Some(0xFFFF)
        );
    }

    /// Test trying decoding of non-zero and non-single item vector structures,
    /// which MUST fail with a specific error.
    #[test]
    fn test_option_decode_vec() {
        assert!(Option::<u8>::confined_decode(&[2u8, 0u8, 0u8, 0u8][..])
            .err()
            .is_some());
        assert!(Option::<u8>::confined_decode(&[3u8, 0u8, 0u8, 0u8][..])
            .err()
            .is_some());
        assert!(Option::<u8>::confined_decode(&[0xFFu8, 0u8, 0u8, 0u8][..])
            .err()
            .is_some());
    }

    /// Test for checking the following rule from LNPBP-5:
    ///
    /// Array of any commitment-serializable type T MUST contain strictly less
    /// than `0x10000` items and must encode as 16-bit little-endian value
    /// corresponding to the number of items followed by a direct encoding
    /// of each of the items.
    #[test]
    fn test_vec_encode() {
        let v1: Vec<u8> = vec![0, 13, 0xFF];
        let v2: Vec<u8> = vec![13];
        let v3: Vec<u64> = vec![0, 13, 13, 0x1FF, 0xFFFFFFFFFFFFFFFF];
        let v4: Vec<u8> =
            (0..0x1FFFF).map(|item| (item % 0xFF) as u8).collect();

        let s1 = &[3u8, 0u8, 0u8, 13u8, 0xFFu8][..];
        let s2 = &[1u8, 0u8, 13u8][..];
        let s3 = &[
            5u8, 0u8, 0, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 13, 0,
            0, 0, 0, 0, 0, 0, 0xFF, 1, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ][..];

        assert_eq!(confined_serialize(&v1).unwrap(), s1);
        assert_eq!(confined_serialize(&v2).unwrap(), s2);
        assert_eq!(confined_serialize(&v3).unwrap(), s3);
        assert!(confined_serialize(&v4).err().is_some());

        assert_eq!(Vec::<u8>::confined_decode(s1).unwrap(), v1);
        assert_eq!(Vec::<u8>::confined_decode(s2).unwrap(), v2);
        assert_eq!(Vec::<u64>::confined_decode(s3).unwrap(), v3);
    }
}
