use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::contract::types::ibc::{Height, MerkleRoot};

// NOTE: Without the other side of the bridge being implemented we don't know
// the exact fields within the cosmos consenus state.
// This must be updated in a future
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CosmosConsensusState {
    pub root: MerkleRoot,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CosmosClientState {
    pub latest_height: Height,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct ConsensusState {
    pub code_id: String, // Go serializes []byte to base64 encoded string
    pub data: String,    // Go serializes []byte to base64 encoded string
    pub timestamp: u64,
    pub root: MerkleRoot,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct ClientState {
    pub data: String,    // Go serializes []byte to base64 encoded string
    pub code_id: String, // Go serializes []byte to base64 encoded string

    #[serde(default)]
    pub frozen: bool,
    pub frozen_height: Option<Height>,
    pub latest_height: Option<Height>,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct WasmHeader {
    pub data: String, // Go serializes []byte to base64 encoded string
    pub height: Height,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct Misbehaviour {
    pub code_id: String, // Go serializes []byte to base64 encoded string
    pub client_id: String,
    pub header_1: WasmHeader,
    pub header_2: WasmHeader,
}
