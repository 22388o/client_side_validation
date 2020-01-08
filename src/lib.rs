// LNP/BP Rust Library
// Written in 2019 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

#![crate_name = "lnpbp"]
#![crate_type = "dylib"]
#![crate_type = "rlib"]

// Coding conventions
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
#![deny(unused_mut)]
#![deny(dead_code)]
#![deny(unused_imports)]
// TODO: when we will be ready for the release #![deny(missing_docs)]

pub extern crate bitcoin;

#[macro_use]
pub mod common;
pub mod cmt;
pub mod seals;
//pub mod cvp;
pub mod state;

pub use common::*;
