pub mod types;
pub mod serialization;

use cosmwasm_std::{DepsMut, Deps, MessageInfo, Env};
use cosmwasm_std::{attr, to_vec, Binary};
use cosmwasm_std::{
    HandleResponse, InitResponse, StdError, StdResult
};

use crate::types::header::Header;
use crate::types::state::{StateEntry, StateConfig};
use crate::state::State;
use crate::traits::{FromRlp, ToRlp};
use crate::contract::{
    serialization::from_base64,
    types::wasm::{Height, ClientState, ConsensusState, WasmHeader, ClientStateCallResponse, ClientStateCallResponseResult, MerkleRoot, Misbehaviour},
    types::msg::{HandleMsg, QueryMsg, LatestHeightResponse, CheckMisbehaviourAndUpdateStateResult, InitializeStateResult, CheckHeaderAndUpdateStateResult}
};

pub(crate) fn init(
    _deps: DepsMut,
    _env: Env, // TODO: see there is time included (block time of the current transaction)
    _info: MessageInfo,
    _msg: HandleMsg,
) -> Result<InitResponse, StdError> {
    // The 10-wasm Init method is split into two calls, where the second (via handle())
    // call expects ClientState included in the return.
    //
    // Therefore it's better to execute whole logic in the second call.
    Ok(InitResponse::default())
}

pub(crate) fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, StdError> {
    match msg {
        HandleMsg::InitializeState {
            consensus_state,
            me,
        } => {
            init_contract(deps, env, info, consensus_state, me)
        },

        HandleMsg::CheckHeaderAndUpdateState {
            header,
            consensus_state,
            me,
        } => update_contract(deps, env, me, consensus_state, header),

        HandleMsg::CheckProposedHeaderAndUpdateState {
            header,
            consensus_state,
            me,
        } => update_contract(deps, env, me, consensus_state, header),

        HandleMsg::CheckMisbehaviourAndUpdateState {
            me,
            misbehaviour,
            consensus_state1,
            consensus_state2,
        } => check_misbehaviour(deps, env, me, misbehaviour, consensus_state1, consensus_state2),
    }
}

pub(crate) fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::LatestHeight {} => {
            // TODO: How is this endpoint called?
            let out = Binary(vec![1,2,3]);

            Ok(out)
        }
    }
}

fn init_contract(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    consensus_state: ConsensusState,
    me: ClientState,
) -> Result<HandleResponse, StdError> {
    // Unmarshal initial state entry (ie. validator set, epoch_size etc.)
    let last_state_entry: StateEntry = from_base64(&consensus_state.data, "msg.initial_state_entry".to_string())?;

    // Verify initial state
    match last_state_entry.verify() {
        Err(e) => return Err(StdError::GenericErr {
            msg: format!("Initial state verification failed. Error: {}", e)
        }),
        _ => {}
    }

    // Update the state
    let response_data = Binary(
        to_vec(&InitializeStateResult {
            me,
            result: ClientStateCallResponseResult {
                is_valid: true,
                err_msg: String::from(""),
            },
        }
    )?);

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "init_block"),
            attr("last_consensus_state_height", last_state_entry.number),
        ],
        data: Some(response_data),
    })
}

fn update_contract(
    _deps: DepsMut,
    _env: Env,
    me: ClientState,
    consensus_state: ConsensusState,
    header: WasmHeader,
) -> Result<HandleResponse, StdError> {
    // Unmarshal header
    let header: Header = from_base64(&header.data, "msg.header".to_string())?;

    // Unmarshal state entry
    let last_state_entry: StateEntry = from_base64(&consensus_state.data, "msg.last_state_entry".to_string())?;

    // Unmarshal state config
    let state_config: StateConfig = from_base64(&me.data, "msg.state_config".to_string())?;

    // Ingest new header
    let mut state: State = State::from_entry(last_state_entry, state_config);
    match state.insert_header(&header) {
        Err(e) => return Err(StdError::GenericErr {
            msg: format!("Unable to ingest header. Error: {}", e)
        }),
        _ => {}
    }

    // Update the state
    let new_client_state = me.clone();
    let new_consensus_state = ConsensusState{
        code_id: consensus_state.code_id,
        data: base64::encode(state.entry().to_rlp().as_slice()),
        timestamp: header.time,
        root: MerkleRoot {
            hash: base64::encode(header.root.to_vec().as_slice()),
        },
        r#type: consensus_state.r#type,
    };

    let response_data = Binary(
        to_vec(&CheckHeaderAndUpdateStateResult {
            new_client_state,
            new_consensus_state,
            result: ClientStateCallResponseResult {
                is_valid: true,
                err_msg: "".to_string(),
            },
        })?
    );

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "update_block"),
            attr("last_consensus_state_height", state.entry().number),
        ],
        data: Some(response_data),
    })
}

pub fn check_misbehaviour(
    _deps: DepsMut,
    _env: Env,
    me: ClientState,
    misbehaviour: Misbehaviour,
    consensus_state1: ConsensusState,
    consensus_state2: ConsensusState,
) -> Result<HandleResponse, StdError> {
    // NOTE: We could add some sanity checks for CeloHeader (ie. check if all significant fields
    // are populated etc). Though the verification uses only consensus state (validator set),
    // header hash and aggregated_seal, so it might not be necessary.

    // The header heights are expected to be the same
    if misbehaviour.header_1.height != misbehaviour.header_2.height {
        return Err(StdError::GenericErr {
            msg: format!(
                "misbehaviour header heights differ, {} != {}",
                misbehaviour.header_1.height, misbehaviour.header_2.height
            )
        });
    }

    // If client is already frozen at earlier height than misbehaviour, return with error
    if me.frozen && me.frozen_height.is_some() &&
       me.frozen_height.unwrap() <= misbehaviour.header_1.height 
    {
        return Err(StdError::GenericErr {
            msg: format!(
                "client is already frozen at earlier height {} than misbehaviour height {}",
                me.frozen_height.unwrap(), misbehaviour.header_1.height
            )
        });
    }

    // Check the validity of the two conflicting headers against their respective
    // trusted consensus states
    check_misbehaviour_header(1, &me, &consensus_state1, &misbehaviour.header_1)?;
    check_misbehaviour_header(2, &me, &consensus_state2, &misbehaviour.header_2)?;

    // Store the new state
    let mut new_client_state = me.clone();
    new_client_state.frozen = true;
    new_client_state.frozen_height = Some(misbehaviour.header_1.height);

    let response_data = Binary(
        to_vec(&CheckMisbehaviourAndUpdateStateResult {
            new_client_state,
            result: ClientStateCallResponseResult {
                is_valid: true,
                err_msg: String::from(""),
            },
        }
    )?);

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "verify_misbehaviour"),
            attr("height", misbehaviour.header_1.height),
        ],
        data: Some(response_data),
    })
}

pub fn check_misbehaviour_header(
    num: u16,
    me: &ClientState,
    consensus_state: &ConsensusState,
    header: &WasmHeader
) -> Result<(), StdError> {
    // Unmarshal header
    let header: Header = from_base64(&header.data, "msg.header".to_string())?;

    // Unmarshal state entry
    let last_state_entry: StateEntry = from_base64(&consensus_state.data, "msg.last_state_entry".to_string())?;

    // Unmarshal state config
    let state_config: StateConfig = from_base64(&me.data, "msg.state_config".to_string())?;

    // Verify header
    let state: State = State::from_entry(last_state_entry, state_config);
    match state.verify_header(&header) {
        Err(e) => return Err(StdError::GenericErr {
            msg: format!("Failed to verify header num: {} against it's consensus state. Error: {}", num, e)
        }),
        _ => return Ok(())
    }
}
