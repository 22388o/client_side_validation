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

//! Base commit-verify scheme interface with extension allowing to create
//! embedded commitments (commit-embed-verify).

/// Trait for commit-verify scheme. A message for the commitment may be any
/// structure that can be represented as a byte array (i.e. implements
/// `AsRef<[u8]>`).
pub trait CommitVerify<M>
where
    Self: Eq + Sized,
{
    /// Creates a commitment to a byte representation of a given message
    fn commit(msg: &M) -> Self;

    /// Verifies commitment against the message; default implementation just
    /// repeats the commitment to the message and check it against the `self`.
    #[inline]
    fn verify(&self, msg: &M) -> bool { Self::commit(msg) == *self }
}

/// Trait for a failable version of commit-verify scheme. A message for the
/// commitment may be any structure that can be represented as a byte array
/// (i.e. implements `AsRef<[u8]>`).
pub trait TryCommitVerify<M>
where
    Self: Eq + Sized,
{
    /// Error type that may be reported during [`TryCommitVerify::try_commit`]
    /// and [`TryCommitVerify::try_verify`] procedures
    type Error: std::error::Error;

    /// Tries to create commitment to a byte representation of a given message
    fn try_commit(msg: &M) -> Result<Self, Self::Error>;

    /// Tries to verify commitment against the message; default implementation
    /// just repeats the commitment to the message and check it against the
    /// `self`.
    #[inline]
    fn try_verify(&self, msg: &M) -> Result<bool, Self::Error> {
        Ok(Self::try_commit(msg)? == *self)
    }
}

/// Trait for *embed-commit-verify scheme*, where some data structure (named
/// *container*) may commit to existing *message* (producing *commitment* data
/// structure) in such way that the original message can't be restored from the
/// commitment, however the fact of the commitment may be deterministically
/// checked when the message is *revealed* against the original container.
///
/// To use *embed-commit-verify scheme* one needs to implement this trait for
/// the commitment data structure and provide it (through associated types)
/// with the used container type.
///
/// Operations with *embed-commit-verify scheme* may be represented in form of
/// `EmbedCommit: (Container, Message) -> Commitment` and
/// `Verify: (Commitment, Container, Message) -> bool`; the original container
/// is required for the verification procedure.
///
/// This trait is heavily used in **deterministic bitcoin commitments**
pub trait EmbedCommitVerify<M>
where
    Self: Sized + Eq,
{
    /// External container type that will be used to host commitment to a
    /// message
    type Container: Clone;
    /// Error type that may be reported during
    /// [`EmbedCommitVerify::embed_commit``] procedure
    type Error: std::error::Error;

    /// Creates a commitment and embeds it into the provided container returning
    /// `Self` containing both message commitment and all additional data
    /// required to reconstruct the original container
    fn embed_commit(
        container: &mut Self::Container,
        msg: &M,
    ) -> Result<Self, Self::Error>;

    /// Verifies commitment against the message; default implementation just
    /// takes original `container`, repeats the commitment to the message and
    /// check it against the `self`.
    ///
    /// Verification is a failable procedure returning bool. The difference
    /// between returning `Ok(false)` and `Err(_)` is the following:
    /// * `Err(_)`: validation was not possible due to container data structure-
    ///   related error or some internal error during the validation process. It
    ///   is undefined whether the message corresponds to the commitment.
    /// * `Ok(false)`: validation was performed completely; the message does not
    ///   correspond to the commitment
    /// * `Ok(true)`: validation was performed completely; the message does
    ///   correspond to the commitment
    #[inline]
    fn verify(
        &self,
        container: &Self::Container,
        msg: &M,
    ) -> Result<bool, Self::Error> {
        let mut container = container.clone();
        Ok(match Self::embed_commit(&mut container, msg) {
            Ok(commitment) => commitment == *self,
            Err(_) => false,
        })
    }
}

/// Helpers for writing test functions working with commit-verify scheme
#[cfg(test)]
pub mod test_helpers {
    use core::fmt::Debug;
    use core::hash::Hash;
    use std::collections::HashSet;

    use bitcoin_hashes::hex::FromHex;

    use super::*;

    /// Generates a set of messages for testing purposes
    ///
    /// All of these messages MUST produce different commitments, otherwise the
    /// commitment algorithm is not collision-resistant
    pub fn gen_messages() -> Vec<Vec<u8>> {
        vec![
            // empty message
            b"".to_vec(),
            // zero byte message
            b"\x00".to_vec(),
            // text message
            b"test".to_vec(),
            // text length-extended message
            b"test*".to_vec(),
            // short binary message
            Vec::from_hex("deadbeef").unwrap(),
            // length-extended version
            Vec::from_hex("deadbeef00").unwrap(),
            // prefixed version
            Vec::from_hex("00deadbeef").unwrap(),
            // serialized public key as text
            b"0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798".to_vec(),
            // the same public key binary data
            Vec::from_hex("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
                .unwrap(),
            // different public key
            Vec::from_hex("02f9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9")
                .unwrap(),
        ]
    }

    /// Runs round-trip of commitment and verification for a given set of
    /// messages
    pub fn commit_verify_suite<MSG, CMT>(messages: Vec<MSG>)
    where
        MSG: AsRef<[u8]> + Eq,
        CMT: CommitVerify<MSG> + Eq + Hash + Debug,
    {
        messages.iter().fold(
            HashSet::<CMT>::with_capacity(messages.len()),
            |mut acc, msg| {
                let commitment = CMT::commit(msg);

                // Commitments MUST be deterministic: each message should
                // produce unique commitment
                (1..10).for_each(|_| {
                    assert_eq!(CMT::commit(msg), commitment);
                });

                // Testing verification
                assert!(commitment.verify(msg));

                messages.iter().for_each(|m| {
                    // Testing that commitment verification succeeds only
                    // for the original message and fails for the rest
                    assert_eq!(commitment.verify(m), m == msg);
                });

                acc.iter().for_each(|cmt| {
                    // Testing that verification against other commitments
                    // returns `false`
                    assert!(!cmt.verify(msg));
                });

                // Detecting collision
                assert!(acc.insert(commitment));

                acc
            },
        );
    }

    /// Runs round-trip of commitment-embed-verify for a given set of messages
    /// and provided container
    pub fn embed_commit_verify_suite<MSG, CMT>(
        messages: Vec<MSG>,
        container: &mut CMT::Container,
    ) where
        MSG: AsRef<[u8]> + Eq,
        CMT: EmbedCommitVerify<MSG> + Eq + Hash + Debug,
    {
        messages.iter().fold(
            HashSet::<CMT>::with_capacity(messages.len()),
            |mut acc, msg| {
                let commitment = CMT::embed_commit(container, msg).unwrap();

                // Commitments MUST be deterministic: each message should
                // produce unique commitment
                (1..10).for_each(|_| {
                    assert_eq!(
                        CMT::embed_commit(container, msg).unwrap(),
                        commitment
                    );
                });

                // Testing verification
                assert!(commitment.verify(container, msg).unwrap());

                messages.iter().for_each(|m| {
                    // Testing that commitment verification succeeds only
                    // for the original message and fails for the rest
                    assert_eq!(
                        commitment.verify(container, m).unwrap(),
                        m == msg
                    );
                });

                acc.iter().for_each(|cmt| {
                    // Testing that verification against other commitments
                    // returns `false`
                    assert!(!cmt.verify(container, msg).unwrap());
                });

                // Detecting collision
                assert!(acc.insert(commitment));

                acc
            },
        );
    }
}

#[cfg(test)]
mod test {
    use core::fmt::Debug;
    use core::hash::Hash;

    use bitcoin_hashes::sha256d;

    use super::test_helpers::*;
    use super::*;

    #[derive(Debug, Display, Error)]
    #[display(Debug)]
    struct Error;
    #[derive(Clone, PartialEq, Eq, Debug, Hash)]
    struct DummyHashCommitment(sha256d::Hash);
    impl<T> CommitVerify<T> for DummyHashCommitment
    where
        T: AsRef<[u8]>,
    {
        fn commit(msg: &T) -> Self {
            Self(bitcoin_hashes::Hash::hash(msg.as_ref()))
        }
    }

    #[derive(Clone, PartialEq, Eq, Debug, Hash)]
    struct DummyVec(Vec<u8>);
    impl<T> EmbedCommitVerify<T> for DummyVec
    where
        T: AsRef<[u8]>,
    {
        type Container = DummyVec;
        type Error = Error;

        fn embed_commit(
            container: &mut Self::Container,
            msg: &T,
        ) -> Result<Self, Self::Error> {
            let mut result = container.0.clone();
            result.extend(msg.as_ref());
            Ok(DummyVec(result))
        }
    }

    #[test]
    fn test_commit_verify() {
        commit_verify_suite::<Vec<u8>, DummyHashCommitment>(gen_messages());
    }

    #[test]
    fn test_embed_commit() {
        embed_commit_verify_suite::<Vec<u8>, DummyVec>(
            gen_messages(),
            &mut DummyVec(vec![]),
        );
    }
}
