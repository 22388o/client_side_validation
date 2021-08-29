// LNP/BP client-side-validation foundation libraries implementing LNPBP
// specifications & standards (LNPBP-4, 7, 8, 9, 42, 81)
//
// Written in 2019-2021 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the Apache 2.0 License along with this
// software. If not, see <https://opensource.org/licenses/Apache-2.0>.

use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use amplify::proc_attr::{
    ArgValue, ArgValueReq, AttrReq, LiteralClass, ParametrizedAttr, ValueClass,
};
use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::spanned::Spanned;
use syn::{
    AngleBracketedGenericArguments, Error, Field, GenericArgument, Ident,
    LitInt, Path, PathArguments, PathSegment, Result, Type, TypePath,
};

pub const CRATE: &str = "crate";
pub const SKIP: &str = "skip";
pub const REPR: &str = "repr";
pub const VALUE: &str = "value";
pub const BY_ORDER: &str = "by_order";
pub const BY_VALUE: &str = "by_value";
pub const USE_TLV: &str = "use_tlv";
pub const TLV: &str = "tlv";
pub const UNKNOWN_TLVS: &str = "unknown_tlvs";

const EXPECT: &str =
    "amplify_syn is broken: requirements for crate arg are not satisfied";

#[derive(Clone)]
pub struct EncodingDerive {
    pub use_crate: Path,
    pub skip: bool,
    pub by_order: bool,
    pub value: Option<LitInt>,
    pub repr: Ident,
    /// `None` if TLVs are not allowed at the struct level with
    /// `#[strict_encoding(use_tlv)]` attribute
    pub tlv: Option<TlvDerive>,
}

#[derive(Clone, Copy)]
pub enum TlvDerive {
    None,
    Typed(u16),
    Unknown,
}

impl EncodingDerive {
    pub fn with(
        attr: &mut ParametrizedAttr,
        is_global: bool,
        is_enum: bool,
        use_tlv: bool,
    ) -> Result<EncodingDerive> {
        let mut map = if is_global {
            map! {
                CRATE => ArgValueReq::with_default(ident!(strict_encoding)),
                USE_TLV => ArgValueReq::with_default(true)
            }
        } else {
            map! {
                SKIP => ArgValueReq::Prohibited,
                TLV => ArgValueReq::Optional(ValueClass::Literal(LiteralClass::Int)),
                UNKNOWN_TLVS => ArgValueReq::with_default(true)
            }
        };

        if is_enum {
            map.insert(BY_ORDER, ArgValueReq::Prohibited);
            map.insert(BY_VALUE, ArgValueReq::Prohibited);
            map.insert(USE_TLV, ArgValueReq::Prohibited);
            map.insert(TLV, ArgValueReq::Prohibited);
            map.insert(UNKNOWN_TLVS, ArgValueReq::Prohibited);
            if is_global {
                map.insert(REPR, ArgValueReq::with_default(ident!(u8)));
            } else {
                map.insert(
                    VALUE,
                    ArgValueReq::Optional(ValueClass::Literal(
                        LiteralClass::Int,
                    )),
                );
            }
        }

        attr.check(AttrReq::with(map))?;

        if attr.args.contains_key(BY_VALUE) && attr.args.contains_key(BY_ORDER)
        {
            return Err(Error::new(
                Span::call_site(),
                "`by_value` and `by_order` attributes can't be present \
                 together",
            ));
        }

        let repr: Ident = attr
            .args
            .get(REPR)
            .cloned()
            .map(TryInto::try_into)
            .transpose()
            .expect(EXPECT)
            .unwrap_or_else(|| ident!(u8));

        match repr.to_string().as_str() {
            "u8" | "u16" | "u32" | "u64" => {}
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "`repr` requires integer type identifier",
                ))
            }
        }

        let use_crate = attr
            .args
            .get(CRATE)
            .cloned()
            .unwrap_or_else(|| ArgValue::from(ident!(strict_encoding)))
            .try_into()
            .expect(EXPECT);

        let value = attr
            .args
            .get(VALUE)
            .cloned()
            .map(LitInt::try_from)
            .transpose()
            .expect(EXPECT);

        let skip = attr.args.get("skip").is_some();

        let by_order = !attr.args.contains_key("by_value");

        let tlv = TlvDerive::with(attr, is_global, use_tlv)?;

        Ok(EncodingDerive {
            use_crate,
            skip,
            by_order,
            value,
            repr,
            tlv,
        })
    }
}

impl TlvDerive {
    pub fn with(
        attr: &mut ParametrizedAttr,
        is_global: bool,
        use_tlv: bool,
    ) -> Result<Option<TlvDerive>> {
        if !use_tlv
            && !attr
                .args
                .get(USE_TLV)
                .cloned()
                .map(bool::try_from)
                .transpose()
                .expect(EXPECT)
                .unwrap_or_default()
        {
            if attr.args.contains_key(TLV)
                || attr.args.contains_key(UNKNOWN_TLVS)
            {
                return Err(Error::new(
                    Span::call_site(),
                    "TLV-related attributes are allowed only when global \
                     `use_tlv` attribute is set",
                ));
            }
            return Ok(None);
        }

        if attr.args.contains_key(TLV) && attr.args.contains_key(UNKNOWN_TLVS) {
            return Err(Error::new(
                Span::call_site(),
                "`tlv` and `unknown_tlvs` attributes are mutually exclusive",
            ));
        }

        if attr.args.contains_key(SKIP) {
            return Err(Error::new(
                Span::call_site(),
                "presence of TLV attribute for the skipped field does not \
                 make sense",
            ));
        }

        let mut tlv = if let Some(tlv) = attr
            .args
            .get(TLV)
            .cloned()
            .map(LitInt::try_from)
            .transpose()
            .expect(EXPECT)
        {
            Some(TlvDerive::Typed(tlv.base10_parse()?))
        } else if attr.args.contains_key(UNKNOWN_TLVS) {
            Some(TlvDerive::Unknown)
        } else {
            None
        };

        if tlv.is_none() && is_global {
            tlv = Some(TlvDerive::None)
        }

        Ok(tlv)
    }

    pub fn process(
        &self,
        field: &Field,
        name: TokenStream2,
        fields: &mut Vec<TokenStream2>,
        tlvs: &mut BTreeMap<u16, TokenStream2>,
        aggregator: &mut Option<TokenStream2>,
    ) -> Result<()> {
        match self {
            TlvDerive::None => {
                fields.push(name);
                Ok(())
            }

            TlvDerive::Typed(type_no) => if let Type::Path(TypePath {
                path,
                ..
            }) = &field.ty
            {
                if let Some(PathSegment { ident, .. }) = path.segments.last() {
                    if *ident == ident!(Option) {
                        let n = name.to_string();
                        if tlvs.insert(*type_no, name).is_some() {
                            return Err(Error::new(
                                field.span(),
                                format!(
                                    "reused TLV type constant {} for field \
                                     `{}`",
                                    type_no, n
                                ),
                            ));
                        } else {
                            Ok(())
                        }
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
            .map_err(|_| {
                Error::new(field.span(), "TLV fields must be optionals in type")
            }),

            TlvDerive::Unknown => {
                if let Type::Path(TypePath { path, .. }) = &field.ty {
                    if aggregator.is_some() {
                        return Err(Error::new(
                            field.span(),
                            "unknown TLVs aggregator can be present only once",
                        ));
                    }
                    if let Some(PathSegment {
                        ident,
                        arguments:
                            PathArguments::AngleBracketed(
                                AngleBracketedGenericArguments { args, .. },
                            ),
                    }) = path.segments.last()
                    {
                        if *ident == ident!(BTreeMap) && args.len() == 2 {
                            match (&args[0], &args[1]) {
                                (
                                    GenericArgument::Type(Type::Path(path1)),
                                    GenericArgument::Type(Type::Path(path2)),
                                ) if path1.path.is_ident(&ident!(u16))
                                    && path2
                                        .path
                                        .segments
                                        .last()
                                        .unwrap()
                                        .ident
                                        == ident!(Box) =>
                                {
                                    *aggregator = Some(name);
                                    Ok(())
                                }
                                _ => Err(()),
                            }
                        } else {
                            Err(())
                        }
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
                .map_err(|_| {
                    Error::new(
                        field.span(),
                        "unknown TLVs aggregator field must be of \
                         `BTreeMap<u16, Box<[u8]>>` type",
                    )
                })
            }
        }
    }
}
