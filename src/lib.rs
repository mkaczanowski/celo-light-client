pub mod types;
pub mod serialization;
pub mod istanbul;
pub mod state;
pub mod bls;
pub mod traits;
pub mod macros;
pub mod errors;
pub mod contract;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate parity_scale_codec;
extern crate parity_scale_codec_derive;

extern crate rlp;
extern crate num_bigint;
extern crate sha3;
extern crate bls_crypto;
extern crate algebra;
extern crate anomaly;
extern crate thiserror;
extern crate cfg_if;

pub use types::{
    header::Header,
    header::Address,
    header::Hash,
    istanbul::SerializedPublicKey,
    istanbul::IstanbulExtra,
    state::Validator,
    state::StateEntry,
};
pub use istanbul::{
    get_epoch_number,
    get_epoch_last_block_number,
};
pub use state::State;
pub use errors::{Error, Kind};
pub use traits::{
    Storage,
    SerializableStorage,
    FromBytes,
    DefaultFrom,
    ToRlp,
};

pub use contract::*;

/// WASM methods exposed to be used by CosmWasm handler
/// All methods are thin wrapper around actual contract contained in
/// contract module.

#[cfg(target_arch = "wasm32")]
pub use wasm::{handle, init, query};

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::contract;
    use cosmwasm_std::{
        do_handle, do_init, do_query, ExternalApi, ExternalQuerier, ExternalStorage,
    };

    /// WASM Entry point for contract::init
    #[no_mangle]
    pub extern "C" fn init(env_ptr: u32, msg_ptr: u32) -> u32 {
        do_init(
            &contract::init::<ExternalStorage, ExternalApi, ExternalQuerier>,
            env_ptr,
            msg_ptr,
        )
    }

    /// WASM Entry point for contract::handle
    #[no_mangle]
    pub extern "C" fn handle(env_ptr: u32, msg_ptr: u32) -> u32 {
        do_handle(
            &contract::handle::<ExternalStorage, ExternalApi, ExternalQuerier>,
            env_ptr,
            msg_ptr,
        )
    }

    /// WASM Entry point for contract::query
    #[no_mangle]
    pub extern "C" fn query(msg_ptr: u32) -> u32 {
        do_query(
            &contract::query::<ExternalStorage, ExternalApi, ExternalQuerier>,
            msg_ptr,
        )
    }
}
