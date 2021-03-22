use crate::types::header::{Header, Address};
use crate::types::istanbul::IstanbulExtra;
use crate::types::state::{Validator, StateEntry, StateConfig};
use crate::errors::{Error, Kind};
use crate::bls::verify_aggregated_seal;
use crate::istanbul::{is_last_block_of_epoch, get_epoch_number};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use num_bigint::BigInt as Integer;
use num::cast::ToPrimitive;
use num_traits::Zero;

pub struct State {
    entry: StateEntry,
    config: StateConfig,
}

impl State {
    pub fn new(config: StateConfig) -> Self {
        State {
            entry: StateEntry::new(),
            config
        }
    }

    pub fn from_entry(entry: StateEntry, config: StateConfig) -> Self {
        State {
            entry,
            config
        }
    }

    pub fn entry(&self) -> &StateEntry {
        &self.entry 
    }

    pub fn add_validators(&mut self, validators: Vec<Validator>) -> bool {
        let mut new_address_map: HashMap<Address, bool> = HashMap::new();
        
        for validator in validators.iter() {
            new_address_map.insert(validator.address, true);
        }

        // Verify that the validators to add is not already in the valset
        for v in self.entry.validators.iter() {
            if new_address_map.contains_key(&v.address) {
                return false;
            }
        }

        self.entry.validators.extend(validators);

        return true;
    }

    pub fn remove_validators(&mut self, removed_validators: &Integer) -> bool {
        if removed_validators.bits() == 0 {
            return true;
        }

        if removed_validators.bits() > self.entry.validators.len() as u64 {
            return false;
        }

        let filtered_validators: Vec<Validator> = self.entry.validators.iter()
            .enumerate()
            .filter(|(i, _)| removed_validators.bit(*i as u64) == false)
            .map(|(_, v)| v.to_owned())
            .collect();

        self.entry.validators = filtered_validators;

        return true;
    }

    pub fn verify_header(&self, header: &Header) -> Result<(), Error> {
        let header_hash = header.hash()?;
        let extra = IstanbulExtra::from_rlp(&header.extra)?;

        // NOTE: This doesn't work in WASM
        //   https://github.com/rust-lang/rust/issues/48564
        if self.config.verify_header_timestamp {
            let curr_time = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(t) => t.as_secs(),
                Err(e) => return Err(Kind::Unknown.context(e).into()),
            };

            // don't waste time checking blocks from the future
            if header.time > curr_time + self.config.allowed_clock_skew {
                return Err(Kind::FutureBlock.into())
            }
        }

        verify_aggregated_seal(
            header_hash,
            &self.entry.validators,
            extra.aggregated_seal
        )
    }

    pub fn insert_header(&mut self, header: &Header) -> Result<(), Error> {
        let block_num = header.number.to_u64().unwrap();

        if is_last_block_of_epoch(block_num, self.config.epoch_size) {
            // The validator set is about to be updated with epoch header
            self.store_epoch_header(header, self.config.verify_epoch_headers)
        } else {
            // Validator set is not being updated
            self.store_non_epoch_header(header, self.config.verify_epoch_headers)
        }
    }

    fn store_non_epoch_header(&mut self, header: &Header, verify: bool) -> Result<(), Error> {
        // genesis block is valid dead end
        if verify && !header.number.is_zero() {
            self.verify_header(&header)?
        }

        let extra = IstanbulExtra::from_rlp(&header.extra)?;
        let entry = StateEntry {
            // The validator state stays unchanged (ONLY updated with epoch header)
            validators: self.entry.validators.clone(),

            // Update the header related fields
            number: header.number.to_u64().unwrap(),
            timestamp: header.time,
            hash: header.hash()?,
            aggregated_seal: extra.aggregated_seal.clone(),
        };

        self.update_state_entry(entry)
    }

    fn store_epoch_header(&mut self, header: &Header, verify: bool) -> Result<(), Error> {
        // genesis block is valid dead end
        if verify && !header.number.is_zero() {
            self.verify_header(&header)?
        }

        let header_hash = header.hash()?;
        let extra = IstanbulExtra::from_rlp(&header.extra)?;

        // convert istanbul validators into a Validator struct
        let mut validators: Vec<Validator> = Vec::new();
        if extra.added_validators.len() != extra.added_validators_public_keys.len() {
            return Err(Kind::InvalidValidatorSetDiff{msg: "error in combining addresses and public keys"}.into());
        }

        for i in 0..extra.added_validators.len() {
            validators.push(Validator{
                address: extra.added_validators[i].clone(),
                public_key: extra.added_validators_public_keys[i].clone(),
            })
        }

        // apply the header's changeset
        let result_remove = self.remove_validators(&extra.removed_validators);
        if !result_remove {
            return Err(Kind::InvalidValidatorSetDiff{msg: "error in removing the header's removed_validators"}.into());
        }

        let result_add = self.add_validators(validators);
        if !result_add {
            return Err(Kind::InvalidValidatorSetDiff{msg: "error in adding the header's added_validators"}.into());
        }

        let entry = StateEntry {
            number: header.number.to_u64().unwrap(),
            timestamp: header.time,
            validators: self.entry.validators.clone(),
            hash: header_hash,
            aggregated_seal: extra.aggregated_seal,
        };

        self.update_state_entry(entry)
    }

    fn update_state_entry(&mut self, entry: StateEntry) -> Result<(), Error> {
        // NOTE: right now we store only the last state entry but we could add
        // a feature to store X past entries for querying / debugging

        // update local state
        self.entry = entry;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cmp, cmp::Ordering};
    use sha3::{Digest, Keccak256};
    use secp256k1::{rand::rngs::OsRng, Secp256k1, PublicKey, SecretKey};
    use crate::traits::{FromBytes, DefaultFrom, FromRlp};
    use crate::types::header::{Hash, ADDRESS_LENGTH};
    use crate::types::istanbul::{SerializedPublicKey, IstanbulAggregatedSeal};

    macro_rules! string_vec {
        ($($x:expr),*) => (vec![$($x.to_string()),*]);
    }

    struct TestValidatorSet {
        validators: Vec<String>,
        validator_set_diffs: Vec<ValidatorSetDiff>,
        results: Vec<String>,
    }

    struct ValidatorSetDiff {
        added_validators: Vec<String>,
        removed_validators: Vec<String>
    }

    struct AccountPool {
        pub accounts: HashMap<String, (SecretKey, PublicKey)>,
    }

    fn state_config() -> StateConfig {
        StateConfig {
            epoch_size: 123,
            allowed_clock_skew: 123,

            verify_epoch_headers: true,
            verify_non_epoch_headers: true, 
            verify_header_timestamp: true,
        }
    }

    impl AccountPool {
        fn new() -> Self {
            Self {
                accounts: HashMap::new(),
            }
        }

        fn address(&mut self, account: String) -> Address {
            if account == "" {
                return Address::default();
            }

            if !self.accounts.contains_key(&account) {
                self.accounts.insert(account.clone(), generate_key());
            }

            pubkey_to_address(self.accounts.get(&account).unwrap().1)
        }
    }

    #[test]
    fn test_encode_state_entry() {
        let entry = StateEntry {
            validators: vec![
                Validator{
                    address: Address::default(),
                    public_key: SerializedPublicKey::default(),
                },
                Validator{
                    address: Address::default(),
                    public_key: SerializedPublicKey::default(),
                }
            ],
            timestamp: 123456,
            number: 456,
            hash: Hash::default(),
            aggregated_seal: IstanbulAggregatedSeal::new(),
        };

        let encoded = rlp::encode(&entry);
        let decoded = StateEntry::from_rlp(&encoded).unwrap();
        
        assert_eq!(entry, decoded);
    }

    #[test]
    fn test_add_remove() {
        let mut state = State::new(state_config());
        let mut result = state.add_validators(vec![
            Validator{
                address: bytes_to_address(&vec![0x3 as u8]),
                public_key: SerializedPublicKey::default(),
            }
        ]);

        assert_eq!(result, true);

        result = state.add_validators(vec![
            Validator{
                address: bytes_to_address(&vec![0x2 as u8]),
                public_key: SerializedPublicKey::default(),
            },
            Validator{
                address: bytes_to_address(&vec![0x1 as u8]),
                public_key: SerializedPublicKey::default(),
            }
        ]);

        assert_eq!(result, true);
        assert_eq!(state.entry.validators.len(), 3);

        // verify ordering
        let current_addresses: Vec<Address> = state.entry.validators.iter().map(|val| val.address).collect();
        let expecected_addresses: Vec<Address> = vec![
            bytes_to_address(&vec![0x3 as u8]),
            bytes_to_address(&vec![0x2 as u8]),
            bytes_to_address(&vec![0x1 as u8]),
        ];
        assert_eq!(current_addresses, expecected_addresses);

        // remove first validator
        result = state.remove_validators(&Integer::from(1));
        assert_eq!(result, true);
        assert_eq!(state.entry.validators.len(), 2);

        // remove second validator
        result = state.remove_validators(&Integer::from(2));
        assert_eq!(result, true);
        assert_eq!(state.entry.validators.len(), 1);

        // remove third validator
        result = state.remove_validators(&Integer::from(1));
        assert_eq!(result, true);
        assert_eq!(state.entry.validators.len(), 0);
    }

    #[test]
    fn applies_validator_set_changes() {
        let tests = vec![
            // Single validator, empty val set diff
            TestValidatorSet {
                validators: string_vec!["A"],
                validator_set_diffs: vec![
                    ValidatorSetDiff{
                        added_validators: Vec::new(),
                        removed_validators: Vec::new(),
                    }
                ],
                results: string_vec!["A"],
            },
            // Single validator, add two new validators
            TestValidatorSet {
                validators: string_vec!["A"],
                validator_set_diffs: vec![
                    ValidatorSetDiff{
                        added_validators: string_vec!["B", "C"],
                        removed_validators: Vec::new(),
                    }
                ],
                results: string_vec!["A", "B", "C"],
            },
            // Two validator, remove two validators
            TestValidatorSet {
                validators: string_vec!["A", "B"],
                validator_set_diffs: vec![
                    ValidatorSetDiff{
                        added_validators: Vec::new(),
                        removed_validators: string_vec!["A", "B"],
                    }
                ],
                results: string_vec![],
            },
            // Three validator, add two validators and remove two validators
            TestValidatorSet {
                validators: string_vec!["A", "B", "C"],
                validator_set_diffs: vec![
                    ValidatorSetDiff{
                        added_validators: string_vec!["D", "E"],
                        removed_validators: string_vec!["B", "C"],
                    }
                ],
                results: string_vec!["A", "D", "E"],
            },
            // Three validator, add two validators and remove two validators.  Second header will add 1 validators and remove 2 validators.
            TestValidatorSet {
                validators: string_vec!["A", "B", "C"],
                validator_set_diffs: vec![
                    ValidatorSetDiff{
                        added_validators: string_vec!["D", "E"],
                        removed_validators: string_vec!["B", "C"],
                    },
                    ValidatorSetDiff{
                        added_validators: string_vec!["F"],
                        removed_validators: string_vec!["A", "D"],
                    }
                ],
                results: string_vec!["F", "E"],
            },
        ];

        for test in tests {
            let mut accounts = AccountPool::new();
            let mut state = State::new(state_config());

            let validators = convert_val_names_to_validators(&mut accounts, test.validators);
            state.add_validators(validators.clone());

            for diff in test.validator_set_diffs {
                let added_validators = convert_val_names_to_validators(&mut accounts, diff.added_validators);
                let removed_validators = convert_val_names_to_removed_validators(&mut accounts, &state.entry.validators, diff.removed_validators);

                state.remove_validators(&removed_validators);
                state.add_validators(added_validators);
            }

            let results = convert_val_names_to_validators(&mut accounts, test.results);
            assert_eq!(compare(state.entry.validators, results), Ordering::Equal);
        }
    }

    pub fn compare(a: Vec<Validator>, b :Vec<Validator>) -> cmp::Ordering {
        let mut sorted_a = a.clone();
        let mut sorted_b = b.clone();

        sorted_a.sort_by(|a, b| b.address.cmp(&a.address));
        sorted_b.sort_by(|a, b| b.address.cmp(&a.address));

        sorted_a.iter()
            .zip(sorted_b)
            .map(|(x, y)| x.address.cmp(&y.address))
            .find(|&ord| ord != cmp::Ordering::Equal)
            .unwrap_or(a.len().cmp(&b.len()))
    }

    fn pubkey_to_address(p: PublicKey) -> Address {
       let pub_bytes = p.serialize_uncompressed();
       let digest = &Keccak256::digest(&pub_bytes[1..])[12..];

       Address::from_bytes(digest).unwrap().to_owned()
    }

    fn bytes_to_address(bytes: &[u8]) -> Address {
        let mut v = vec![0x0; ADDRESS_LENGTH - bytes.len()];
        v.extend_from_slice(bytes);

        Address::from_bytes(&v).unwrap().to_owned()
    }

    fn generate_key() -> (SecretKey, PublicKey) {
        let mut rng = OsRng::new().expect("OsRng");
        let secp = Secp256k1::new();

        secp.generate_keypair(&mut rng)
    }

    fn convert_val_names_to_validators(accounts: &mut AccountPool, val_names: Vec<String>) -> Vec<Validator> {
        val_names.iter().map(|name| Validator{
            address: accounts.address(name.to_string()),
            public_key: SerializedPublicKey::default(),
        }).collect()
    }

    fn convert_val_names_to_removed_validators(accounts: &mut AccountPool, old_validators: &[Validator], val_names: Vec<String>) -> Integer {
        let mut bitmap = Integer::from(0);
        for v in val_names {
            for j in 0..old_validators.len() {
                if &accounts.address(v.to_string()) == &old_validators.get(j).unwrap().address {
                    bitmap.set_bit(j as u64, true);
                }
            }
        }

        bitmap
    }
}
