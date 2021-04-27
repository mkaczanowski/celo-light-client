use crate::contract::serialization::to_generic_err;
use cosmwasm_std::{from_slice, StdError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

// This file defines core IBC structures.
// We can't use protobuf compiled structs because:
//  * Go serializes []byte as base64 string (additional operation is required to decode to Vec<u8>)
//  * Deriving JsonSchema via prost isn't possible
//
// Also, the Light Client package shouldn't mantain copy of IBC protobuf files etc. In the long run
// there should be upstream library (ie. `ibc-rs`) that would expose those structs.

// Origin: ibc.core.connection.v1
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConnectionEnd {
    client_id: String,
    versions: Vec<Version>,
    state: i32,
    counterparty: Counterparty,
    delay_period: u64,
}

// Origin: ibc.core.connection.v1
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Counterparty {
    pub client_id: String,
    pub connection_id: String,
    pub prefix: MerklePrefix,
}

// Origin: ibc.core.connection.v1
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Version {
    pub identifier: String,
    pub features: Vec<String>,
}

// Origin: ibc.core.channel.v1
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Channel {
    pub state: i32,
    pub ordering: i32,
    pub counterparty: Counterparty,
    pub connection_hops: Vec<String>,
    pub version: String,
}

// Origin: ibc.core.commitment.v1
#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MerklePrefix {
    pub key_prefix: String // Go serializes []byte to base64 encoded string
}

// Origin: ibc.core.commitment.v1
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MerkleRoot {
    pub hash: String, // Go serializes []byte to base64 encoded string
}

// Origin: ibc.core.commitment.v1
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MerklePath {
    pub key_path: Vec<String>,
}

// Origin: ibc.core.commitment.v1
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct MerkleProof {
    pub proofs: Vec<ics23::CommitmentProof>,
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug, JsonSchema)]
pub struct Height {
    #[serde(default)]
    pub revision_number: u64,

    #[serde(default)]
    pub revision_height: u64,
}

impl PartialOrd for Height {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Height {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.revision_number < other.revision_number {
            Ordering::Less
        } else if self.revision_number > other.revision_number {
            Ordering::Greater
        } else if self.revision_height < other.revision_height {
            Ordering::Less
        } else if self.revision_height > other.revision_height {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl std::fmt::Display for Height {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "revision: {}, height: {}",
            self.revision_number, self.revision_height
        )
    }
}

pub fn apply_prefix(
    prefix: &MerklePrefix,
    mut path: Vec<String>,
) -> Result<MerklePath, StdError> {
    if prefix.key_prefix.len() == 0 {
        return Err(to_generic_err("empty prefix"));
    }

    let prefix_from_base64 = base64::decode(&prefix.key_prefix).map_err(|e| to_generic_err(e))?;
    let decoded_prefix = std::str::from_utf8(&prefix_from_base64).map_err(|e| to_generic_err(e))?;

    let mut result: Vec<String> = vec![decoded_prefix.to_string()];
    result.append(&mut path);
    Ok(MerklePath { key_path: result })
}

// Origin: cosmos-sdk/x/ibc/core/23-commitment/types/merkle.go
pub fn verify_membership(
    proof: &MerkleProof,
    specs: &[ics23::ProofSpec],
    root: &Vec<u8>,
    keys: &MerklePath,
    mut value: Vec<u8>,
    index: usize,
) -> Result<bool, StdError> {
    let mut subroot = value.clone();

    for (i, commitment_proof) in proof.proofs.iter().skip(index).enumerate() {
        if let Some(ex) = get_exist_proof(commitment_proof) {
            subroot = ics23::calculate_existence_root(&ex).map_err(|e| to_generic_err(e))?;
            let key = match keys.key_path.get(keys.key_path.len() - 1 - i) {
                Some(key) => key,
                None => return Err(StdError::generic_err("could not retrieve key bytes")),
            };

            if !ics23::verify_membership(
                &commitment_proof,
                &specs[i],
                &subroot,
                key.as_bytes(),
                &value,
            ) {
                return Err(StdError::generic_err(format!(
                    "membership proof failed to verify membership of value: {:?} in subroot: {:?}",
                    value, subroot
                )));
            }

            value = subroot.clone();
        } else {
            return Err(StdError::generic_err(
                "expected proof type: ics23::ExistenceProof",
            ));
        }
    }

    if !root.iter().eq(subroot.iter()) {
        return Err(StdError::generic_err(format!(
            "proof did not commit to expected root: {:?}, got: {:?}",
            root, subroot
        )));
    }

    Ok(true)
}

fn get_exist_proof<'a>(proof: &'a ics23::CommitmentProof) -> Option<&'a ics23::ExistenceProof> {
    match &proof.proof {
        Some(ics23::commitment_proof::Proof::Exist(ex)) => Some(ex),
        _ => None,
    }
}
