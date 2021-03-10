pub mod msg;
mod state;
mod storage;

use std::ops::Deref;
use cosmwasm_std::{DepsMut, Deps, MessageInfo, Env};
use cosmwasm_std::{attr, to_vec, Binary};
use cosmwasm_std::{Storage};
use cosmwasm_std::{
    HandleResponse, InitResponse, StdError, StdResult
};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};


use crate::types::header::Header;
use crate::types::state::StateEntry;
use crate::state::State;
use crate::traits::{FromRlp, ToRlp, SerializableStorage};
use crate::contract::{
    state::ContractState,
    storage::WasmStorage,
    msg::{Height, ClientState, HandleMsg, ConsensusState, WasmHeader, InitMsg, LatestHeightResponse, QueryMsg, ClientStateCallResponse, ClientStateCallResponseResult}
};

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_MESSAGES: &[u8] = b"messages";

pub const KEY_STATE_CONS: &[u8] = b"consensus_state";
pub const KEY_STATE_CLIENT: &[u8] = b"client_state";

pub const NUM_COLS: u32 = 1;

pub use crate::traits::Storage as CeloStorage;

fn contract_state(storage: &mut dyn Storage) -> Singleton<ContractState> {
    singleton(storage, KEY_STATE_CLIENT)
}

fn read_only_contract_state(
    storage: &dyn Storage,
) -> ReadonlySingleton<ContractState> {
    singleton_read(storage, KEY_STATE_CLIENT)
}

pub(crate) fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: HandleMsg,
) -> Result<InitResponse, StdError> {
    let init_msg: InitMsg = match msg {
        HandleMsg::InitializeState {
            consensus_state,
            me: _,
        } => {
            let init_msg_bytes = match base64::decode(&consensus_state.data) {
                Ok(bytes) => bytes,
                Err(e) => {
                    return Err(StdError::ParseErr {
                        target_type: "consensus_state.data".to_string(),
                        msg: format!("Unable to decode data from base64 string. Error: {}", e)
                    })
                }
            };

            match InitMsg::from_rlp(&init_msg_bytes) {
                Ok(header) => header,
                Err(e) => {
                    return Err(StdError::ParseErr {
                        target_type: "consensus_state.data".to_string(),
                        msg: format!("Unable to decode InitMsg from rlp. Error: {}", e)
                    })
                }
            }
        },

        _ => {
            return Err(StdError::GenericErr {
                msg: "invalid enum type, expected InitializeState".to_string(),
            });
        }
    };

    // Unmarshal header
    let header = match Header::from_rlp(&init_msg.header) {
        Ok(header) => header,
        Err(e) => {
            return Err(StdError::ParseErr {
                target_type: "msg.header".to_string(),
                msg: e.to_string()
            })
        }
    };

    // Unmarshal initial state entry (ie. validator set, epoch_size etc.)
    let last_state_entry = match StateEntry::from_rlp(&init_msg.initial_state_entry) {
        Ok(header) => header,
        Err(e) => {
            return Err(StdError::ParseErr {
                target_type: "msg.initial_state_entry".to_string(),
                msg: e.to_string(),
            })
        }
    };

    // Initialize state
    let storage = Box::new(WasmStorage::new(NUM_COLS));
    let epoch_size = last_state_entry.epoch;
    let mut state = State::from_entry(last_state_entry, storage);

    // Ingest new header
    match state.insert_header(&header, true) {
        Err(e) => return Err(StdError::GenericErr {
            msg: format!("Unable to ingest header. Error: {}", e)
        }),
        _ => {}
    }

    // Serialize storage
    let encoded = SerializableStorage::<WasmStorage>::serialize(state.storage().deref());

    // Write state
    let new_contract_state = ContractState {
        name: "Test".to_string(),
        epoch_size,
        light_client_data: encoded
    };

    contract_state(deps.storage).save(&new_contract_state)?;

    Ok(InitResponse::default())
}

pub(crate) fn handle(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, StdError> {
    match msg {
        HandleMsg::InitializeState {
            consensus_state: _,
            me,
        } => {
            let out = Binary(to_vec(&ClientStateCallResponse {
                me: me.clone(),
                new_client_state: me,
                new_consensus_state: ConsensusState {
                    data: "test".to_string()
                },
                result: ClientStateCallResponseResult {
                    is_valid: true,
                    err_msg: "".to_string(),
                },
            })?);

            Ok(HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "block"),
                    //attr("last_header", state.entry().number),
                ],
                data: Some(out),
            })
        },
        HandleMsg::CheckAndUpdateClientState {
            header,
            me,
        } => try_block(deps, env, me, header),
    }
}

pub(crate) fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LatestHeight {} => {
            let state = read_only_contract_state(deps.storage).load()?;

            let storage: Box<WasmStorage> = match WasmStorage::deserialize(&state.light_client_data) {
                Ok(storage) => storage,
                Err(e) => {
                    return Err(StdError::ParseErr {
                        target_type: "msg.storage".to_string(),
                        msg: format!("Failed to decode storage: {}", e.to_string()),
                    })
                }
            };

            // Restore state from storage
            let mut state = State::new(state.epoch_size, storage);
            let last_epoch = match state.restore() {
                Ok(epoch) => epoch,
                Err(e) => {
                    return Err(StdError::GenericErr {
                        msg: format!("Failed to restore state from storage: {}", e),
                    });
                }
            };

            let out = Binary(to_vec(&LatestHeightResponse {
                last_header_height: state.entry().number,
                last_header_hash: state.entry().hash,
                last_epoch,
                validator_set: state.entry().validators.to_rlp(),
            })?);

            Ok(out)
        }
    }
}

fn try_block(
    deps: DepsMut,
    _env: Env,
    me: ClientState,
    header: WasmHeader,
) -> Result<HandleResponse, StdError> {
    // Unmarshal rlp-base64-string to bytes
    let header_bytes = match base64::decode(&header.data) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err(StdError::ParseErr {
                target_type: "msg.header".to_string(),
                msg: e.to_string(),
            })
        }
    };
    // Unmarshal header
    let header = match Header::from_rlp(header_bytes.as_slice()) {
        Ok(block) => block,
        Err(e) => {
            return Err(StdError::ParseErr {
                target_type: "msg.header".to_string(),
                msg: format!("Unable to construct header from header bytes. Error: {}", e),
            })
        }
    };

    // Restore last state from wasm storage
    let current_contract_state = contract_state(deps.storage).load()?;

    // Decode storage
    let storage: Box<WasmStorage> = match WasmStorage::deserialize(&current_contract_state.light_client_data) {
        Ok(storage) => storage,
        Err(e) => {
            return Err(StdError::ParseErr {
                target_type: "msg.light_client_data".to_string(),
                msg: format!("Failed to decode storage: {}", e.to_string())
            })
        }
    };

    // Restore state from storage
    let mut state = State::new(current_contract_state.epoch_size, storage);
    match state.restore() {
        Ok(epoch) => epoch,
        Err(e) => {
            return Err(StdError::GenericErr {
                msg: format!("Failed to restore state from storage: {}", e),
            });
        }
    };

    // Ingest new header
    match state.insert_header(&header, true) {
        Err(e) => return Err(StdError::GenericErr {
            msg: format!("Unable to ingest header (update call). Error: {}", e),
        }),
        _ => {}
    }

    // Store new contract
    let encoded = SerializableStorage::<WasmStorage>::serialize(state.storage().deref());
    let new_contract_state = ContractState {
        name: current_contract_state.name,
        epoch_size: current_contract_state.epoch_size,
        light_client_data: encoded,
    };

    contract_state(deps.storage).save(&new_contract_state)?;

    // prepare response
    let mut new_me = me.clone();
    new_me.latest_height = Some(Height{
        revision_number: 0,
        revision_height: state.entry().number,
    });

    let out = Binary(to_vec(&ClientStateCallResponse {
        me,
        new_client_state: new_me,
        new_consensus_state: ConsensusState {
            data: "test".to_string()
        },
        result: ClientStateCallResponseResult {
            is_valid: true,
            err_msg: "".to_string(),
        },
    })?);

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "block"),
            attr("last_header", state.entry().number),
        ],
        data: Some(out),
    };

    return Ok(res);
}
