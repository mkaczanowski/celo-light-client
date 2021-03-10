use crate::types::header::Hash;
use crate::errors::{Error, Kind};
use crate::traits::{
    ToRlp,
    FromRlp
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use rlp::{Rlp, Encodable, Decodable, RlpStream, DecoderError};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub header: Vec<u8>,
    pub initial_state_entry: Vec<u8>,
}

impl Encodable for InitMsg {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(2);

        s.append_list(&self.header.as_ref());
        s.append_list(&self.initial_state_entry.as_ref());
    }
}

impl ToRlp for InitMsg {
    fn to_rlp(&self) -> Vec<u8> {
        rlp::encode(self)
    }
}

impl Decodable for InitMsg {
        fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
            Ok(InitMsg{
                header: rlp.at(0)?.as_list()?,
                initial_state_entry: rlp.at(1)?.as_list()?,
            })
        }
}

impl FromRlp for InitMsg {
    fn from_rlp(bytes: &[u8]) -> Result<Self, Error> {
        match rlp::decode(&bytes) {
            Ok(data) => Ok(data),
            Err(_) => Err(Kind::GenericSerializationError.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ClientStateData {
    pub max_clock_drift: u64,
    // TODO: add more fields here
}

impl Encodable for ClientStateData {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(1);

        s.append(&self.max_clock_drift);
    }
}

impl ToRlp for ClientStateData {
    fn to_rlp(&self) -> Vec<u8> {
        rlp::encode(self)
    }
}

impl Decodable for ClientStateData {
        fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
            Ok(ClientStateData{
                max_clock_drift: rlp.val_at(0)?,
            })
        }
}

impl FromRlp for ClientStateData {
    fn from_rlp(bytes: &[u8]) -> Result<Self, Error> {
        match rlp::decode(&bytes) {
            Ok(data) => Ok(data),
            Err(_) => Err(Kind::GenericSerializationError.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct ConsensusState {
    pub data: String, // []byte is encoded as hex string via Go = base64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub struct WasmHeader {
    pub data: String,
    pub height: Height,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    InitializeState {
        consensus_state: ConsensusState,
        me: ClientState,
    },
    CheckAndUpdateClientState {
        header: WasmHeader,
        me: ClientState
    }
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
pub struct Height {
    #[serde(default)]
    pub revision_number: u64,

    #[serde(default)]
    pub revision_height: u64,
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
