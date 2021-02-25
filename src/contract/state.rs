use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ContractState {
    pub name: String,
    pub epoch_size: u64,
    pub light_client_data: Vec<u8>,
}
