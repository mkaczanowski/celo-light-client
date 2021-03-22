use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct MerkleRoot {
    pub hash: String, // []byte is encoded as hex string via Go = base64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct ConsensusState {
    pub code_id: String, // []byte is encoded as hex string via Go = base64
    pub data: String, // []byte is encoded as hex string via Go = base64
    pub timestamp: u64,
    pub root: MerkleRoot,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct ClientState {
    pub data: String, // Go serializes []byte to base64 encoded string
    pub code_id: String, // Go serializes []byte to base64 encoded string

    #[serde(default)]
    pub frozen: bool,
    pub frozen_height: Option<Height>,
    pub latest_height: Option<Height>,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct WasmHeader {
    pub data: String, // []byte is encoded as hex string via Go = base64
    pub height: Height,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct ClientStateCallResponse {
    pub me: ClientState,
    pub result: ClientStateCallResponseResult,
    pub new_client_state: ClientState,
    pub new_consensus_state: ConsensusState,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct ClientStateCallResponseResult {
    pub is_valid: bool,
    pub err_msg: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct Misbehaviour {
    pub code_id: String, // []byte is encoded as hex string via Go = base64
    pub client_id: String,
    pub header_1: WasmHeader,
    pub header_2: WasmHeader,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CheckMisbehaviourAndUpdateStateResult {
    pub result: ClientStateCallResponseResult,
    pub new_client_state: ClientState,
}
