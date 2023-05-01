// Client-side-validation foundation libraries.
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2019-2023 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2023 LNP/BP Standards Association. All rights reserved.
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

use amplify::confinement::{SmallOrdMap, SmallVec};
use amplify::num::{u256, u4};
use amplify::Wrapper;

#[cfg(feature = "rand")]
pub use self::commit::Error;
use crate::merkle::MerkleNode;
use crate::mpc::atoms::Leaf;
use crate::mpc::{Commitment, Message, MessageMap, Proof, ProtocolId, MERKLE_LNPBP4_TAG};
use crate::{strategies, CommitStrategy, CommitmentId, Conceal, LIB_NAME_COMMIT_VERIFY};

type OrderedMap = SmallOrdMap<u16, (ProtocolId, Message)>;

/// Complete information about LNPBP-4 merkle tree.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[derive(StrictDumb, StrictType, StrictEncode, StrictDecode)]
#[strict_type(lib = LIB_NAME_COMMIT_VERIFY)]
pub struct MerkleTree {
    /// Tree depth (up to 16).
    pub(super) depth: u4,

    /// Entropy used for placeholders.
    pub(super) entropy: u64,

    /// Map of the messages by their respective protocol ids
    pub(super) messages: MessageMap,

    pub(super) map: OrderedMap,
}

impl Proof for MerkleTree {}

impl CommitStrategy for MerkleTree {
    type Strategy = strategies::ConcealStrict;
}

impl CommitmentId for MerkleTree {
    const TAG: [u8; 32] = *b"urn:lnpbp:lnpbp0004:tree:v01#23A";
    type Id = Commitment;
}

impl MerkleTree {
    pub fn root(&self) -> MerkleNode {
        let iter = (0..self.width()).into_iter().map(|pos| {
            self.map
                .get(&pos)
                .map(|(protocol, msg)| Leaf::inhabited(*protocol, *msg))
                .unwrap_or_else(|| Leaf::entropy(self.entropy, pos))
        });
        let leaves = SmallVec::try_from_iter(iter).expect("u16-bound size");
        MerkleNode::merklize(MERKLE_LNPBP4_TAG.to_be_bytes(), &leaves)
    }
}

impl Conceal for MerkleTree {
    type Concealed = MerkleNode;

    fn conceal(&self) -> Self::Concealed { self.root() }
}

#[cfg(feature = "rand")]
mod commit {
    use amplify::confinement::Confined;
    use rand::{thread_rng, RngCore};

    use super::*;
    use crate::mpc::MultiSource;
    use crate::{TryCommitVerify, UntaggedProtocol};

    /// Errors generated during multi-message commitment process by
    /// [`MerkleTree::try_commit`]
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Error, Debug, Display)]
    #[display(doc_comments)]
    pub enum Error {
        /// can't create commitment for an empty message list and zero tree
        /// depth.
        Empty,

        /// number of messages ({0}) for LNPBP-4 commitment which exceeds the
        /// protocol limit of 2^16
        TooManyMessages(usize),

        /// the provided number of messages can't fit LNPBP-4 commitment size
        /// limits for a given set of protocol ids.
        CantFitInMaxSlots,
    }

    impl TryCommitVerify<MultiSource, UntaggedProtocol> for MerkleTree {
        type Error = Error;

        fn try_commit(source: &MultiSource) -> Result<Self, Error> {
            use std::collections::BTreeMap;

            if source.min_depth == u4::ZERO && source.messages.is_empty() {
                return Err(Error::Empty);
            }
            if source.messages.len() > 2usize.pow(u4::MAX.to_u8() as u32) {
                return Err(Error::TooManyMessages(source.messages.len()));
            }

            let entropy = thread_rng().next_u64();

            let mut map = BTreeMap::<u16, (ProtocolId, Message)>::new();

            let mut depth = source.min_depth;
            loop {
                let width = 2usize.pow(depth.to_u8() as u32) as u16;
                if source.messages.iter().all(|(protocol, message)| {
                    let pos = protocol_id_pos(*protocol, width);
                    map.insert(pos, (*protocol, *message)).is_none()
                }) {
                    break;
                }

                depth += 1;
            }

            Ok(MerkleTree {
                depth,
                messages: source.messages.clone(),
                entropy,
                map: Confined::try_from(map).expect("MultiSource type guarantees"),
            })
        }
    }
}

pub(super) fn protocol_id_pos(protocol_id: ProtocolId, width: u16) -> u16 {
    let rem = u256::from_le_bytes((*protocol_id).into_inner()) % u256::from(width as u64);
    rem.low_u64() as u16
}

impl MerkleTree {
    /// Computes position for a given `protocol_id` within the tree leaves.
    pub fn protocol_id_pos(&self, protocol_id: ProtocolId) -> u16 {
        protocol_id_pos(protocol_id, self.width())
    }

    /// Computes the width of the merkle tree.
    pub fn width(&self) -> u16 { 2usize.pow(self.depth.to_u8() as u32) as u16 }

    pub fn depth(&self) -> u4 { self.depth }

    pub fn entropy(&self) -> u64 { self.entropy }
}
