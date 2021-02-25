mod msg;
mod state;
mod storage;

use std::ops::Deref;
use cosmwasm_std::{log, Env};
use cosmwasm_std::{to_vec, Binary};
use cosmwasm_std::{Api, Extern, ReadonlyStorage, Storage};
use cosmwasm_std::{
    HandleResponse, HandleResult, InitResponse, InitResult, Querier, QueryResult, StdError,
};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

use crate::types::header::Header;
use crate::types::state::StateEntry;
use crate::state::State;
use crate::traits::{FromRlp, ToRlp, SerializableStorage};
use crate::contract::{
    state::ContractState,
    storage::WasmStorage,
    msg::{HandleMsg, InitMsg, LatestHeightResponse, QueryMsg}
};

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_MESSAGES: &[u8] = b"messages";

pub const KEY_STATE_CONS: &[u8] = b"consensus_state";
pub const KEY_STATE_CLIENT: &[u8] = b"client_state";

pub const NUM_COLS: u32 = 1;

pub use crate::traits::Storage as CeloStorage;

fn contract_state<S: Storage>(storage: &mut S) -> Singleton<S, ContractState> {
    singleton(storage, KEY_STATE_CLIENT)
}

fn read_only_contract_state<S: ReadonlyStorage>(
    storage: &S,
) -> ReadonlySingleton<S, ContractState> {
    singleton_read(storage, KEY_STATE_CLIENT)
}

pub(crate) fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> InitResult {
    // Check name, symbol, decimals
    if !is_valid_identifier(&msg.name) {
        return Err(StdError::ParseErr {
            target: "msg.name".to_string(),
            msg: "Name is not in the expected format (8-20 lowercase UTF-8 bytes)".to_string(),
            backtrace: None,
        });
    }

    // Unmarshal header
    let header = match Header::from_rlp(&msg.header) {
        Ok(header) => header,
        Err(e) => {
            return Err(StdError::ParseErr {
                target: "msg.header".to_string(),
                msg: e.to_string(),
                backtrace: None,
            })
        }
    };

    // Unmarshal initial state entry (ie. validator set, epoch_size etc.)
    let last_state_entry = match StateEntry::from_rlp(&msg.initial_state_entry) {
        Ok(header) => header,
        Err(e) => {
            return Err(StdError::ParseErr {
                target: "msg.initial_state_entry".to_string(),
                msg: e.to_string(),
                backtrace: None,
            })
        }
    };

    // Initialize state
    let storage = Box::new(WasmStorage::new(NUM_COLS));
    let epoch_size = last_state_entry.epoch;
    let mut state = State::from_entry(last_state_entry, storage);

    // Ingest new header
    match state.insert_header(&header, false) { // TODO: we should validate new header (wasm isssue)
        Err(e) => return Err(StdError::GenericErr {
            msg: format!("Unable to ingest header. Error: {}", e),
            backtrace: None,
        }),
        _ => {}
    }

    // Serialize storage
    let encoded = SerializableStorage::<WasmStorage>::serialize(state.storage().deref());

    // Write state
    let new_contract_state = ContractState {
        name: msg.name,
        epoch_size,
        light_client_data: encoded
    };

    contract_state(&mut deps.storage).save(&new_contract_state)?;

    Ok(InitResponse::default())
}

pub(crate) fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::UpdateClient {
            header,
        } => try_block(deps, env, &header),
    }
}

pub(crate) fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
    match msg {
        QueryMsg::LatestHeight {} => {
            let state = read_only_contract_state(&deps.storage).load()?;

            let storage: Box<WasmStorage> = match WasmStorage::deserialize(&state.light_client_data) {
                Ok(storage) => storage,
                Err(e) => {
                    return Err(StdError::ParseErr {
                        target: "msg.storage".to_string(),
                        msg: format!("Failed to decode storage: {}", e.to_string()),
                        backtrace: None,
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
                        backtrace: None,
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

fn try_block<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    header_hex: &String,
) -> HandleResult {
    // Unmarshal rlp-hex-string to bytes
    let header_bytes = match hex::decode(&header_hex) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err(StdError::ParseErr {
                target: "msg.header".to_string(),
                msg: e.to_string(),
                backtrace: None,
            })
        }
    };

    // Unmarshal header
    let header = match Header::from_rlp(header_bytes.as_slice()) {
        Ok(block) => block,
        Err(e) => {
            return Err(StdError::ParseErr {
                target: "msg.header".to_string(),
                msg: format!("Unable to construct header from header bytes. Error: {}", e),
                backtrace: None,
            })
        }
    };

    // Restore last state from wasm storage
    let current_contract_state = contract_state(&mut deps.storage).load()?;

    // Decode storage
    let storage: Box<WasmStorage> = match WasmStorage::deserialize(&current_contract_state.light_client_data) {
        Ok(storage) => storage,
        Err(e) => {
            return Err(StdError::ParseErr {
                target: "msg.light_client_data".to_string(),
                msg: format!("Failed to decode storage: {}", e.to_string()),
                backtrace: None,
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
                backtrace: None,
            });
        }
    };

    // Ingest new header
    match state.insert_header(&header, false) {
        Err(e) => return Err(StdError::GenericErr {
            msg: format!("Unable to ingest header (update call). Error: {}", e),
            backtrace: None,
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

    contract_state(&mut deps.storage).save(&new_contract_state)?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "block"),
            log("last_header", state.entry().number),
        ],
        data: None,
    };

    return Ok(res);
}

fn is_valid_identifier(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 8 || bytes.len() > 20 {
        return false; // length invalid
    }
    for byte in bytes {
        if byte > &122 || byte < &97 {
            return false; // not lowercase ascii
        }
    }
    return true;
}
