pub mod types;
pub mod serialization;
pub mod istanbul;
pub mod state;
pub mod bls;
pub mod traits;
pub mod macros;
pub mod errors;

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
    state::Snapshot,
    state::Config
};
pub use istanbul::{
    get_epoch_number,
    get_epoch_last_block_number,
};
pub use state::State;
pub use errors::{Error, Kind};
pub use traits::{
    FromBytes,
    DefaultFrom,
    ToRlp,
    FromRlp
};

#[cfg(all(feature = "wasm_contract"))]
pub mod contract;

#[cfg(all(feature = "wasm_contract"))]
pub use contract::*;

#[cfg(all(feature = "wasm_contract", target_arch = "wasm32"))]
cosmwasm_std::create_entry_points!(contract);
