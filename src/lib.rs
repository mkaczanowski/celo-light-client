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

extern crate rlp;
extern crate num_bigint;
extern crate sha3;
extern crate bls_crypto;
extern crate algebra;
extern crate anomaly;
extern crate thiserror;

pub use types::{
    header::Header,
    header::Address,
    header::Hash,
    istanbul::SerializedPublicKey,
    istanbul::IstanbulExtra,
    state::Validator,
    state::StateEntry,
    state::StateConfig
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
    FromRlp
};

pub use contract::*;

/// WASM methods exposed to be used by CosmWasm handler
/// All methods are thin wrapper around actual contract contained in
/// contract module.

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);
