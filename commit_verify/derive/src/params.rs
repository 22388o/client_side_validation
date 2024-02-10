// Client-side-validation foundation libraries.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2019-2024 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2024 LNP/BP Standards Association. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use amplify_syn::{
    ArgValueReq, AttrReq, DataType, FieldKind, ListReq, ParametrizedAttr, TypeClass, ValueClass,
};
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::{DeriveInput, Error, Expr, Path, Result};

const ATTR: &str = "commit_encode";
const ATTR_CRATE: &str = "crate";
const ATTR_CONCEAL: &str = "conceal";
const ATTR_STRATEGY: &str = "strategy";
const ATTR_STRATEGY_COMMIT: &str = "propagate";
const ATTR_STRATEGY_STRICT: &str = "strict";
const ATTR_STRATEGY_TRANSPARENT: &str = "transparent";
const ATTR_STRATEGY_INTO_U8: &str = "into_u8";
const ATTR_MERKLIZE: &str = "merklize";
const ATTR_SKIP: &str = "skip";

pub struct ContainerAttr {
    pub commit_crate: Path,
    pub strategy: StrategyAttr,
    pub conceal: bool,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum StrategyAttr {
    CommitEncoding,
    StrictEncoding,
    ConcealStrictEncoding,
    Transparent,
    IntoU8,
}

impl TryFrom<&Path> for StrategyAttr {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self> {
        match path.to_token_stream().to_string().as_str() {
            ATTR_STRATEGY_COMMIT => Ok(StrategyAttr::CommitEncoding),
            ATTR_STRATEGY_STRICT => Ok(StrategyAttr::StrictEncoding),
            ATTR_STRATEGY_TRANSPARENT => Ok(StrategyAttr::Transparent),
            ATTR_STRATEGY_INTO_U8 => Ok(StrategyAttr::IntoU8),
            unknown => Err(Error::new(
                Span::call_site(),
                format!(
                    "invalid commitment encoding value for `strategy` attribute `{unknown}`; only \
                     `{ATTR_STRATEGY_TRANSPARENT}`, `{ATTR_STRATEGY_INTO_U8}`, \
                     `{ATTR_STRATEGY_COMMIT}`, or `{ATTR_STRATEGY_STRICT}`  are allowed"
                ),
            )),
        }
    }
}

impl StrategyAttr {
    pub fn to_ident(self) -> Ident {
        match self {
            StrategyAttr::CommitEncoding => {
                panic!("StrategyAttr::CommitEncoding must be derived manually")
            }
            StrategyAttr::StrictEncoding => ident!(Strict),
            StrategyAttr::ConcealStrictEncoding => ident!(ConcealStrict),
            StrategyAttr::Transparent => ident!(IntoInner),
            StrategyAttr::IntoU8 => ident!(IntoU8),
        }
    }
}

pub struct FieldAttr {
    pub merklize: Option<Expr>,
    pub skip: bool,
}

impl TryFrom<ParametrizedAttr> for ContainerAttr {
    type Error = Error;

    fn try_from(mut params: ParametrizedAttr) -> Result<Self> {
        let mut req = AttrReq::with(map![
            ATTR_CRATE => ArgValueReq::optional(TypeClass::Path),
            ATTR_STRATEGY => ArgValueReq::optional(TypeClass::Path),
        ]);
        req.path_req = ListReq::maybe_one(path!(conceal));
        params.check(req)?;

        let path = params
            .arg_value(ATTR_STRATEGY)
            .unwrap_or_else(|_| path!(propagate));

        let mut strategy = StrategyAttr::try_from(&path)?;
        let conceal = params.has_verbatim(ATTR_CONCEAL);
        if conceal && strategy == StrategyAttr::StrictEncoding {
            strategy = StrategyAttr::ConcealStrictEncoding
        }
        Ok(ContainerAttr {
            commit_crate: params
                .arg_value(ATTR_CRATE)
                .unwrap_or_else(|_| path!(commit_verify)),
            strategy,
            conceal,
        })
    }
}

impl FieldAttr {
    pub fn with(mut params: ParametrizedAttr, _kind: FieldKind) -> Result<Self> {
        let mut req = AttrReq::with(map![
            ATTR_MERKLIZE => ArgValueReq::optional(ValueClass::Expr),
        ]);
        req.path_req = ListReq::maybe_one(path!(skip));
        params.check(req)?;

        Ok(FieldAttr {
            skip: params.has_verbatim(ATTR_SKIP),
            merklize: params.arg_value(ATTR_MERKLIZE).ok(),
        })
    }
}

pub struct CommitDerive {
    pub data: DataType,
    pub conf: ContainerAttr,
}

impl TryFrom<DeriveInput> for CommitDerive {
    type Error = Error;

    fn try_from(input: DeriveInput) -> Result<Self> {
        let params = ParametrizedAttr::with(ATTR, &input.attrs)?;
        let conf = ContainerAttr::try_from(params)?;
        let data = DataType::with(input, ident!(commit_encode))?;
        Ok(Self { data, conf })
    }
}
