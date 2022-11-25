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

/// Macro simplifying encoding for a given list of items
#[macro_export]
macro_rules! confined_encode_list {
    ( $encoder:ident; $($item:expr),+ ) => {
        {
            $(
                $item.confined_encode($encoder)?;
            )+
        }
    };
}

/// Macro simplifying decoding of a structure with a given list of fields
#[macro_export]
macro_rules! confined_decode_self {
    ( $decoder:ident; $($item:ident),+ ) => {
        {
            Self {
            $(
                $item: $crate::ConfinedDecode::confined_decode($decoder)?,
            )+
            }
        }
    };
}

#[macro_export]
/// Implements confined encoding for a hash type
macro_rules! hash_encoding {
    ($ty:ty) => {
        impl ConfinedEncode for $ty {
            fn confined_encode(&self, e: &mut impl Write) -> Result<(), Error> {
                encode(self, e)
            }
        }
        impl ConfinedDecode for $ty {
            fn confined_decode(d: &mut impl Read) -> Result<Self, Error> {
                let mut buf = [0u8; <$ty as Hash>::LEN];
                d.read_exact(&mut buf)?;
                Ok(<$ty as Hash>::from_slice(&buf)
                    .expect("bitcoin hashes inner structure is broken"))
            }
        }
    };
}
