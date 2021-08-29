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

use amplify::proc_attr::ParametrizedAttr;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, TokenStreamExt};
use syn::spanned::Spanned;
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Ident,
    ImplGenerics, Index, LitStr, Result, TypeGenerics, WhereClause,
};

use crate::param::{EncodingDerive, TlvDerive, CRATE, REPR, USE_TLV};
use crate::TlvEncoding;

pub fn decode_derive(
    attr_name: &'static str,
    trait_name: Ident,
    decode_name: Ident,
    deserialize_name: Ident,
    input: DeriveInput,
    tlv_encoding: TlvEncoding,
) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();
    let ident_name = &input.ident;

    let global_param = ParametrizedAttr::with(attr_name, &input.attrs)?;

    match input.data {
        Data::Struct(data) => decode_struct_impl(
            attr_name,
            &trait_name,
            &decode_name,
            &deserialize_name,
            data,
            ident_name,
            global_param,
            impl_generics,
            ty_generics,
            where_clause,
            tlv_encoding,
        ),
        Data::Enum(data) => decode_enum_impl(
            attr_name,
            &trait_name,
            &decode_name,
            &deserialize_name,
            data,
            ident_name,
            global_param,
            impl_generics,
            ty_generics,
            where_clause,
        ),
        Data::Union(_) => Err(Error::new_spanned(
            &input,
            format!("Deriving `{}` is not supported in unions", trait_name),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn decode_struct_impl(
    attr_name: &'static str,
    trait_name: &Ident,
    decode_name: &Ident,
    deserialize_name: &Ident,
    data: DataStruct,
    ident_name: &Ident,
    mut global_param: ParametrizedAttr,
    impl_generics: ImplGenerics,
    ty_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
    tlv_encoding: TlvEncoding,
) -> Result<TokenStream2> {
    let encoding = EncodingDerive::with(&mut global_param, true, false, false)?;

    if tlv_encoding == TlvEncoding::Denied && encoding.tlv.is_some() {
        return Err(Error::new(
            ident_name.span(),
            format!("TLV extensions are not allowed in `{}`", attr_name),
        ));
    }

    let inner_impl = match data.fields {
        Fields::Named(ref fields) => decode_fields_impl(
            attr_name,
            trait_name,
            decode_name,
            deserialize_name,
            ident_name,
            &fields.named,
            global_param,
            false,
            tlv_encoding,
        )?,
        Fields::Unnamed(ref fields) => decode_fields_impl(
            attr_name,
            trait_name,
            decode_name,
            deserialize_name,
            ident_name,
            &fields.unnamed,
            global_param,
            false,
            tlv_encoding,
        )?,
        Fields::Unit => quote! {},
    };

    let import = encoding.use_crate;

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #import::#trait_name for #ident_name #ty_generics #where_clause {
            #[inline]
            fn #decode_name<D: ::std::io::Read>(mut d: D) -> Result<Self, #import::Error> {
                use #import::#trait_name;
                #inner_impl
            }
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn decode_enum_impl(
    attr_name: &'static str,
    trait_name: &Ident,
    decode_name: &Ident,
    deserialize_name: &Ident,
    data: DataEnum,
    ident_name: &Ident,
    mut global_param: ParametrizedAttr,
    impl_generics: ImplGenerics,
    ty_generics: TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> Result<TokenStream2> {
    let encoding = EncodingDerive::with(&mut global_param, true, true, false)?;
    let repr = encoding.repr;

    let mut inner_impl = TokenStream2::new();

    for (order, variant) in data.variants.iter().enumerate() {
        let mut local_param =
            ParametrizedAttr::with(attr_name, &variant.attrs)?;

        // First, test individual attribute
        let _ = EncodingDerive::with(&mut local_param, false, true, false)?;
        // Second, combine global and local together
        let mut combined = global_param.clone().merged(local_param.clone())?;
        combined.args.remove(REPR);
        combined.args.remove(CRATE);
        let encoding = EncodingDerive::with(&mut combined, false, true, false)?;

        if encoding.skip {
            continue;
        }

        let field_impl = match variant.fields {
            Fields::Named(ref fields) => decode_fields_impl(
                attr_name,
                trait_name,
                decode_name,
                deserialize_name,
                ident_name,
                &fields.named,
                local_param,
                true,
                TlvEncoding::Denied,
            )?,
            Fields::Unnamed(ref fields) => decode_fields_impl(
                attr_name,
                trait_name,
                decode_name,
                deserialize_name,
                ident_name,
                &fields.unnamed,
                local_param,
                true,
                TlvEncoding::Denied,
            )?,
            Fields::Unit => TokenStream2::new(),
        };

        let ident = &variant.ident;
        let value = match (encoding.value, encoding.by_order) {
            (Some(val), _) => val.to_token_stream(),
            (None, true) => Index::from(order as usize).to_token_stream(),
            (None, false) => quote! { Self::#ident as #repr },
        };

        inner_impl.append_all(quote_spanned! { variant.span() =>
            x if x == #value => {
                Self::#ident {
                    #field_impl
                }
            }
        });
    }

    let import = encoding.use_crate;
    let enum_name = LitStr::new(&ident_name.to_string(), Span::call_site());

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #import::#trait_name for #ident_name #ty_generics #where_clause {
            fn #decode_name<D: ::std::io::Read>(mut d: D) -> Result<Self, #import::Error> {
                use #import::#trait_name;
                Ok(match #repr::#decode_name(&mut d)? {
                    #inner_impl
                    unknown => Err(#import::Error::EnumValueNotKnown(#enum_name, unknown as usize))?
                })
            }
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn decode_fields_impl<'a>(
    attr_name: &'static str,
    trait_name: &Ident,
    decode_name: &Ident,
    deserialize_name: &Ident,
    ident_name: &Ident,
    fields: impl IntoIterator<Item = &'a Field>,
    mut parent_param: ParametrizedAttr,
    is_enum: bool,
    tlv_encoding: TlvEncoding,
) -> Result<TokenStream2> {
    let mut stream = TokenStream2::new();

    let use_tlv = parent_param.args.contains_key(USE_TLV);
    parent_param.args.remove(CRATE);
    parent_param.args.remove(USE_TLV);
    let parent_attr =
        EncodingDerive::with(&mut parent_param.clone(), false, is_enum, false)?;
    let import = parent_attr.use_crate;

    let mut skipped_fields = vec![];
    let mut strict_fields = vec![];
    let mut tlv_fields = bmap! {};
    let mut tlv_aggregator = None;

    for (index, field) in fields.into_iter().enumerate() {
        let mut local_param = ParametrizedAttr::with(attr_name, &field.attrs)?;

        // First, test individual attribute
        let _ =
            EncodingDerive::with(&mut local_param, false, is_enum, use_tlv)?;
        // Second, combine global and local together
        let mut combined = parent_param.clone().merged(local_param)?;
        let encoding =
            EncodingDerive::with(&mut combined, false, is_enum, use_tlv)?;

        let name = field
            .ident
            .as_ref()
            .map(Ident::to_token_stream)
            .unwrap_or_else(|| Index::from(index).to_token_stream());

        if encoding.skip {
            skipped_fields.push(name);
            continue;
        }

        encoding.tlv.unwrap_or(TlvDerive::None).process(
            field,
            name,
            &mut strict_fields,
            &mut tlv_fields,
            &mut tlv_aggregator,
        )?;
    }

    for name in strict_fields {
        stream.append_all(quote_spanned! { Span::call_site() =>
            #name: #import::#trait_name::#decode_name(&mut d)?,
        });
    }

    let mut default_fields = skipped_fields;
    default_fields.extend(tlv_fields.values().cloned());
    default_fields.extend(tlv_aggregator.clone());
    for name in default_fields {
        stream.append_all(quote_spanned! { Span::call_site() =>
            #name: Default::default(),
        });
    }

    if use_tlv {}

    if !is_enum {
        if use_tlv && (!tlv_fields.is_empty() || tlv_aggregator.is_some()) {
            let mut inner = TokenStream2::new();
            for (type_no, name) in tlv_fields {
                inner.append_all(quote_spanned! { Span::call_site() =>
                    #type_no => s.#name = #import::#trait_name::#deserialize_name(bytes)?,
                });
            }

            let mut aggregator = TokenStream2::new();
            if let Some(tlv_aggregator) = tlv_aggregator {
                aggregator = quote_spanned! { Span::call_site() =>
                    _ => { s.#tlv_aggregator.insert(type_no, bytes); },
                };
            };

            stream = match tlv_encoding {
                TlvEncoding::Count => quote_spanned! { Span::call_site() =>
                    let mut s = #ident_name { #stream };
                    let tlvs = ::std::collections::BTreeMap::<usize, Box<[u8]>>::#decode_name(&mut d)?;
                },

                TlvEncoding::Length => quote_spanned! { Span::call_site() =>
                    let mut s = #ident_name { #stream };
                    let mut tlvs: ::std::collections::BTreeMap<usize, Box<[u8]>> = Default::default();
                    let data = Box::<[u8]>::#decode_name(&mut d)?;
                    let iter = data.into_iter();
                    while iter.len() > 0 {
                        let type_no = usize::#decode_name(&mut d)?;
                        let len = usize::#decode_name(&mut d)?;
                        let bytes: Box<[u8]> = iter.clone().take(len).copied().collect();
                        let max = tlvs.keys().max().copied().unwrap_or_default();
                        if type_no > max {
                            return Err(#import::TlvError::Order {
                                read: type_no,
                                max
                            }.into());
                        }
                        if bytes.len() != len {
                            return Err(#import::TlvError::Len {
                                expected: len,
                                actual: bytes.len()
                            }.into());
                        }
                        if tlvs.insert(type_no, bytes).is_some() {
                            return Err(#import::TlvError::Repeated(type_no).into());
                        }
                    }
                },

                TlvEncoding::Denied => unreachable!(
                    "denied TLV encoding is already checked in the caller \
                     method"
                ),
            };

            stream.append_all(quote_spanned! { Span::call_site() =>
                for (type_no, bytes) in tlvs {
                    match type_no {
                        #inner

                        #aggregator
                    }
                }
                Ok(s)
            });
        } else {
            stream = quote_spanned! { Span::call_site() =>
                Ok(#ident_name { #stream })
            };
        }
    }

    Ok(stream)
}
