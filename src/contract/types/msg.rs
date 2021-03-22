use crate::types::header::Hash;
use crate::contract::types::wasm::{WasmHeader, ConsensusState, ClientState, Misbehaviour, ClientStateCallResponseResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    InitializeState {
        consensus_state: ConsensusState,
        me: ClientState,
    },
    CheckHeaderAndUpdateState {
        header: WasmHeader,
        consensus_state: ConsensusState,
        me: ClientState
    },
    CheckProposedHeaderAndUpdateState {
        header: WasmHeader,
        consensus_state: ConsensusState,
        me: ClientState
    },
    CheckMisbehaviourAndUpdateState {
        me: ClientState,
        misbehaviour: Misbehaviour,
        consensus_state1: ConsensusState,
        consensus_state2: ConsensusState,
    }

    //CheckMisbehaviourAndUpdateState: {}, // TODO: via cli 
    /////home/admin/Projects/gaia-upstream/build/gaiad --home "data/.gaiad" tx ibc wasm-client misbehaviour
    // VerifyClientState
    // VerifyClientConsensusState
    // VerifyUpgradeAndUpdateState .. maybe?
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct InitializeStateResult {
    pub result: ClientStateCallResponseResult,
    pub me: ClientState,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CheckHeaderAndUpdateStateResult {
    pub new_client_state: ClientState,
    pub new_consensus_state: ConsensusState,
    pub result: ClientStateCallResponseResult,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct CheckMisbehaviourAndUpdateStateResult {
    pub result: ClientStateCallResponseResult,
    pub new_client_state: ClientState,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryMsg {
    LatestHeight {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct LatestHeightResponse {
    pub last_header_height: u64,
    pub last_header_hash: Hash,
    pub last_epoch: u64,
    pub validator_set: Vec<u8>,
}

