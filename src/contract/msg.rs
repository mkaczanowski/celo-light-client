use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::types::header::Hash;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub header: Vec<u8>,
    pub initial_state_entry: Vec<u8>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    UpdateClient {
        header: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryMsg {
    LatestHeight {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct LatestHeightResponse {
    pub last_header_height: u64,
    pub last_header_hash: Hash,
    pub last_epoch: u64,
    pub validator_set: Vec<u8>,
}
