// LNP/BP client-side-validation foundation libraries implementing LNPBP
// specifications & standards (LNPBP-4, 7, 8, 9, 81)
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

// Coding conventions
#![recursion_limit = "256"]
#![deny(dead_code, missing_docs)]

//! Library implementing **strict encoding** standard, defined by
//! [LNPBP-7](https://github.com/LNP-BP/LNPBPs/blob/master/lnpbp-0007.md).
//! Strict encoding is a binary conservative encoding extensively used in
//! client-side-validation for deterministic portable (platform-independent)
//! serialization of data with a known internal data structure. Strict encoding
//! is a schema-less encoding.
//!
//! Library defines two main traits, [`ConfinedEncode`] and [`ConfinedDecode`],
//! which should be implemented on each type that requires to be represented
//! for client-side-validation. It also defines possible encoding error cases
//! with [`derive@Error`] and provides derivation macros
//! `#[derive(ConfinedEncode, ConfinedDecode)]`, which are a part of
//! `confined_encode_derive` sub-crate and represented by a default feature
//! `derive`. Finally, it implements strict encoding traits for main data types
//! defined by rust standard library and frequently used crates; the latter
//! increases the number of dependencies and thus can be controlled with
//! feature flags:
//! - `derive`: adds derivation macros
//! - `bulletproofs`: pedersen commitments + bulletproofs from
//!   `lnpbp_secp256k1zkp` library.

#[cfg(feature = "derive")]
pub extern crate confined_encoding_derive as derive;
#[cfg(feature = "derive")]
pub use derive::{ConfinedDecode, ConfinedEncode};

#[macro_use]
extern crate amplify;
#[cfg(test)]
#[macro_use]
extern crate confined_encoding_test;
#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;
#[cfg(feature = "bulletproofs")]
extern crate lnpbp_secp256k1zkp as secp256k1zkp;

#[macro_use]
mod macros;

mod check;
mod encodings;
pub mod path;
#[macro_use]
pub mod schema;
mod write;

use std::ops::Range;
use std::string::FromUtf8Error;
use std::{fmt, io};

/// Re-exporting extended read and write functions from bitcoin consensus
/// module so others may use semantic convenience
/// `confined_encode::ReadExt`
pub use ::bitcoin::consensus::encode::{ReadExt, WriteExt};
use amplify::confinement::{Confined, MediumVec, SmallVec};
use amplify::{ascii, confinement, IoError};
pub use encodings::ConfinedTag;

pub use crate::check::{CheckError, CheckedStructBuilder, CheckedWriter};
use crate::schema::{Fields, Ty, Variants};
pub use crate::write::{Builder, ConfinedWrite, StructBuild, Writer};

pub trait ConfinedType {
    /// Type name for the schema
    const TYPE_NAME: &'static str;

    /// Returns type representing confined encoding information
    fn confined_type() -> Ty;
}

/// Binary encoding according to the strict rules that usually apply to
/// consensus-critical data structures. May be used for network communications;
/// in some circumstances may be used for commitment procedures; however it must
/// be kept in mind that sometime commitment may follow "fold" scheme
/// (Merklization or nested commitments) and in such cases this trait can't be
/// applied. It is generally recommended for consensus-related commitments to
/// utilize `CommitVerify`, `TryCommitVerify` and `EmbedCommitVerify` traits  
/// from `commit_verify` module.
pub trait ConfinedEncode: ConfinedType {
    /// Encode with the given [`io::Write`] instance; must return result
    /// with either amount of bytes encoded – or implementation-specific
    /// error type.
    fn confined_encode(&self, e: &mut impl ConfinedWrite) -> Result<(), Error>;

    fn confined_check(&self) -> u16 {
        let mut checker = CheckedWriter::new(Self::confined_type());
        self.confined_encode(&mut checker)
            .expect("checker doesn't fail on encoding");
        checker.size()
    }

    /// Serializes data as a byte array not larger than 64kB (2^16-1 bytes)
    /// using [`ConfinedEncode::confined_encode`] function
    fn confined_serialize<const MAX: usize>(
        &self,
    ) -> Result<Confined<Vec<u8>, 0, MAX>, Error> {
        let buf = Confined::<Vec<u8>, 0, MAX>::new();
        let mut e = Writer::from(buf);
        self.confined_encode(&mut e)?;
        Ok(e.unbox())
    }

    /// Serializes data as a byte array not larger than 64kB (2^16-1 bytes)
    /// using [`ConfinedEncode::confined_encode`] function
    fn confined_serialize_64kb(&self) -> Result<SmallVec<u8>, Error> {
        self.confined_serialize()
    }

    /// Serializes data as a byte array not larger than 16MB (2^24-1 bytes)
    /// using [`ConfinedEncode::confined_encode`] function
    fn confined_serialize_16mb(&self) -> Result<MediumVec<u8>, Error> {
        self.confined_serialize()
    }
}

/// Binary decoding according to the strict rules that usually apply to
/// consensus-critical data structures. May be used for network communications.
/// MUST NOT be used for commitment verification: even if the commit procedure
/// uses [`ConfinedEncode`], the actual commit verification MUST be done with
/// `CommitVerify`, `TryCommitVerify` and `EmbedCommitVerify` traits, which,
/// instead of deserializing (nonce operation for commitments) repeat the
/// commitment procedure for the revealed message and verify it against the
/// provided commitment.
pub trait ConfinedDecode: ConfinedType + Sized {
    /// Decode with the given [`std::io::Read`] instance; must either
    /// construct an instance or return implementation-specific error type.
    fn confined_decode(d: &mut impl io::Read) -> Result<Self, Error>;

    /// Tries to deserializesuper byte array into the current type using
    /// [`ConfinedDecode::confined_decode`]. If there are some data remains in
    /// the buffer once deserialization is completed, fails with
    /// [`Error::DataNotEntirelyConsumed`].
    fn confined_deserialize<const MIN: usize, const MAX: usize>(
        data: &Confined<Vec<u8>, MIN, MAX>,
    ) -> Result<Self, Error> {
        let mut cursor = io::Cursor::new(data.as_inner());
        let me = Self::confined_decode(&mut cursor)?;
        if cursor.position() as usize != data.len() {
            return Err(Error::DataNotEntirelyConsumed);
        }
        Ok(me)
    }

    /// Tries to deserialize byte array into the current type using
    /// [`ConfinedDecode::confined_decode`]. If there are some data remains in
    /// the buffer once deserialization is completed, fails with
    /// [`Error::DataNotEntirelyConsumed`].
    fn confined_deserialize_64bk(data: &SmallVec<u8>) -> Result<Self, Error> {
        Self::confined_deserialize(data)
    }

    /// Tries to deserialize byte array into the current type using
    /// [`ConfinedDecode::confined_decode`]. If there are some data remains in
    /// the buffer once deserialization is completed, fails with
    /// [`Error::DataNotEntirelyConsumed`]. Use `io::Cursor` over the buffer and
    /// [`ConfinedDecode::confined_decode`] to avoid such failures.
    fn confined_deserialize_16mb(data: &MediumVec<u8>) -> Result<Self, Error> {
        Self::confined_deserialize(data)
    }
}

/// Possible errors during strict encoding and decoding process
#[derive(Clone, Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// I/O error during data strict encoding
    #[from(io::Error)]
    Io(IoError),

    /// string data are not in valid UTF-8 encoding.\nDetails: {0}
    #[from]
    Utf8(std::str::Utf8Error),

    /// string data are not in valid ASCII encoding.\nDetails: {0}
    #[from]
    Ascii(ascii::AsAsciiStrError),

    /// Error in the collection confinement (see [`confinement`] module docs).
    #[display(inner)]
    #[from]
    Confinement(confinement::Error),

    /// unsupported value `{1}` for enum `{0}` encountered during decode
    /// operation
    EnumValueNotKnown(&'static str, u8, Variants),

    /// unsupported value `{1}` for union `{0}` encountered during decode
    /// operation
    UnionValueNotKnown(&'static str, u8, Fields),

    /// non-ASCII character {0:#04x}
    NonAsciiChar(u8),

    /// decoding resulted in value `{2}` for type `{0}` that exceeds the
    /// supported range {1:#?}
    ValueOutOfRange(&'static str, Range<u128>, u128),

    /// a repeated value for `{0}` found during set collection deserialization
    RepeatedValue(String),

    /// encoded values are not deterministically ordered: value `{0}` should go
    /// before `{1}`
    BrokenOrder(String, String),

    /// Returned by the convenience method [`ConfinedDecode::confined_decode`]
    /// if not all provided data were consumed during decoding process
    #[display(
        "Data were not consumed entirely during strict decoding procedure"
    )]
    DataNotEntirelyConsumed,

    /// data integrity problem during strict decoding operation.\nDetails: {0}
    DataIntegrityError(String),
}

impl From<Error> for fmt::Error {
    #[inline]
    fn from(_: Error) -> Self { fmt::Error }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self { Error::Utf8(err.utf8_error()) }
}

impl<O> From<ascii::FromAsciiError<O>> for Error {
    fn from(err: ascii::FromAsciiError<O>) -> Self {
        Error::Ascii(err.ascii_error())
    }
}
