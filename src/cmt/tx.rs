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

use bitcoin::{Amount, TxOut, Transaction, hashes::sha256};
use secp256k1::PublicKey;

use crate::common::*;
use super::{*, pubkey::Error};


#[derive(Clone, Eq, PartialEq)]
pub struct TxContainer {
    pub entropy: u32,
    pub tx: Transaction,
    pub container: TxoutContainer,
}

#[derive(Clone, Eq, PartialEq)]
pub struct TxCommitment {
    pub entropy: u32,
    pub tx: Transaction,
    pub tweaked: TxoutCommitment,
    pub original: TxoutContainer,
}

impl<MSG> CommitmentVerify<MSG> for TxCommitment where
    MSG: EmbedCommittable<Self> + EmbedCommittable<TxoutCommitment> + AsSlice
{

    #[inline]
    fn reveal_verify(&self, msg: &MSG) -> bool {
        <Self as EmbeddedCommitment<MSG>>::reveal_verify(&self, msg)
    }
}

impl<MSG> EmbeddedCommitment<MSG> for TxCommitment where
    MSG: EmbedCommittable<Self> + EmbedCommittable<TxoutCommitment> + AsSlice
{
    type Container = TxContainer;
    type Error = Error;

    #[inline]
    fn get_original_container(&self) -> Self::Container {
        let root = match &self.tweaked {
            TxoutCommitment::LockScript(script) => None,
            TxoutCommitment::TapRoot(cmt) => Some(cmt.script_root),
        };
        TxContainer {
            entropy: self.entropy,
            tx: self.tx.clone(),
            container: self.original.clone()
        }
    }

    fn from(container: &Self::Container, msg: &MSG) -> Result<Self, Self::Error> {
        let tx = container.tx.clone();
        let fee = 0; // FIXME: tx.get_fee();
        let entropy = container.entropy;
        let nouts = tx.output.len();
        let vout = (fee + entropy) % (nouts as u32);
        let txout = tx.output[vout as usize].clone();
        let txout_container = container.container.clone();
        let tweaked: TxoutCommitment = EmbeddedCommitment::<MSG>::from(&txout_container, msg)?;
        Ok(Self {
            entropy, tx, original: txout_container, tweaked
        })
    }
}

impl<T> Verifiable<TxCommitment> for T where T: AsSlice { }

impl<T> EmbedCommittable<TxCommitment> for T where T: AsSlice { }


mod test {
    use bitcoin::{*, consensus::encode::deserialize};
    use hex::decode as hex_decode;
    use super::*;

    struct OffchainProof {
        pub entropy: u32,
        pub txout_info: TxoutContainer
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct Message<'a>(&'a str);
    impl AsSlice for Message<'_> {
        fn as_slice(&self) -> &[u8] {
            &self.0.as_bytes()
        }
    }

    #[test]
    fn test_ability_to_commit() {
        let tx = deserialize(hex_decode("020000000001031cfbc8f54fbfa4a33a30068841371f80dbfe166211242213188428f437445c91000000006a47304402206fbcec8d2d2e740d824d3d36cc345b37d9f65d665a99f5bd5c9e8d42270a03a8022013959632492332200c2908459547bf8dbf97c65ab1a28dec377d6f1d41d3d63e012103d7279dfb90ce17fe139ba60a7c41ddf605b25e1c07a4ddcb9dfef4e7d6710f48feffffff476222484f5e35b3f0e43f65fc76e21d8be7818dd6a989c160b1e5039b7835fc00000000171600140914414d3c94af70ac7e25407b0689e0baa10c77feffffffa83d954a62568bbc99cc644c62eb7383d7c2a2563041a0aeb891a6a4055895570000000017160014795d04cc2d4f31480d9a3710993fbd80d04301dffeffffff06fef72f000000000017a91476fd7035cd26f1a32a5ab979e056713aac25796887a5000f00000000001976a914b8332d502a529571c6af4be66399cd33379071c588ac3fda0500000000001976a914fc1d692f8de10ae33295f090bea5fe49527d975c88ac522e1b00000000001976a914808406b54d1044c429ac54c0e189b0d8061667e088ac6eb68501000000001976a914dfab6085f3a8fb3e6710206a5a959313c5618f4d88acbba20000000000001976a914eb3026552d7e3f3073457d0bee5d4757de48160d88ac0002483045022100bee24b63212939d33d513e767bc79300051f7a0d433c3fcf1e0e3bf03b9eb1d70220588dc45a9ce3a939103b4459ce47500b64e23ab118dfc03c9caa7d6bfc32b9c601210354fd80328da0f9ae6eef2b3a81f74f9a6f66761fadf96f1d1d22b1fd6845876402483045022100e29c7e3a5efc10da6269e5fc20b6a1cb8beb92130cc52c67e46ef40aaa5cac5f0220644dd1b049727d991aece98a105563416e10a5ac4221abac7d16931842d5c322012103960b87412d6e169f30e12106bdf70122aabb9eb61f455518322a18b920a4dfa887d30700")
            .unwrap().as_slice()).unwrap();

        let proof = OffchainProof {
            entropy: 0,
            txout_info: TxoutContainer::LockScript(LockScript::from(Script::new()))
        };
        some(tx, proof, &Message("message to commit to"));
    }

    fn some(tx: Transaction, proof: OffchainProof, msg: &Message) {
        let container = TxContainer {
            entropy: proof.entropy,
            tx,
            container: proof.txout_info
        };
        let commitment: TxCommitment = msg.commit_embed(&container).unwrap();
        //let commitment: TxCommitment = EmbeddedCommitment::<Message>::from(&container, msg).unwrap();
        //let verify = commitment.reveal_verify(msg);
        let verify = msg.verify(&commitment);
    }
}