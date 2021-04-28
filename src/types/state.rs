use crate::types::header::{Address, Hash};
use crate::types::istanbul::{IstanbulAggregatedSeal, SerializedPublicKey};
use crate::traits::{ToRlp, FromRlp};
use crate::serialization::rlp::{rlp_field_from_bytes, rlp_list_field_from_bytes};
use crate::errors::{Error, Kind};
use crate::bls::verify_aggregated_seal;

use rlp::{Rlp, Encodable, Decodable, RlpStream, DecoderError};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Validator{
    #[serde(with = "crate::serialization::bytes::hexstring")]
    pub address: Address,

    #[serde(with = "crate::serialization::bytes::hexstring")]
    pub public_key: SerializedPublicKey,
}

impl Encodable for Validator {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(2);

        s.append(&self.address.as_ref());
        s.append(&self.public_key.as_ref());
    }
}

impl Decodable for Validator {
        fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
            Ok(Validator{
                address: rlp_field_from_bytes(&rlp.at(0)?)?,
                public_key: rlp_field_from_bytes(&rlp.at(1)?)?
            })
        }
}

impl ToRlp for Vec<Validator> {
    fn to_rlp(&self) -> Vec<u8> {
        rlp::encode_list(&self)
    }
}

impl FromRlp for Vec<Validator> {
    fn from_rlp(bytes: &[u8]) -> Result<Self, Error> {
        Ok(rlp::decode_list(&bytes))
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StateConfig {
    pub epoch_size: u64,
    pub allowed_clock_skew: u64,
    pub trusting_period: u64,
    pub upgrade_path: Vec<String>,

    pub verify_epoch_headers: bool,
    pub verify_non_epoch_headers: bool,
    pub verify_header_timestamp: bool,

    pub allow_update_after_misbehavior: bool,
    pub allow_update_after_expiry: bool,
}

impl Encodable for StateConfig {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(9);

        s.append(&self.epoch_size);
        s.append(&self.allowed_clock_skew);
        s.append(&self.trusting_period);

        s.begin_list(self.upgrade_path.len());
        for path in self.upgrade_path.iter() {
            s.append(path);
        }

        s.append(&self.verify_epoch_headers);
        s.append(&self.verify_non_epoch_headers);
        s.append(&self.verify_header_timestamp);

        s.append(&self.allow_update_after_misbehavior);
        s.append(&self.allow_update_after_expiry);
    }
}

impl Decodable for StateConfig {
        fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
            let upgrade_path: Result<Vec<String>, DecoderError> = rlp
                .at(3)?
                .iter()
                .map(|r| {
                    r.as_val()
                })
                .collect();

            Ok(StateConfig{
                epoch_size: rlp.val_at(0)?,
                allowed_clock_skew: rlp.val_at(1)?,
                trusting_period: rlp.val_at(2)?,
                upgrade_path: upgrade_path?,
                verify_epoch_headers: rlp.val_at(4)?,
                verify_non_epoch_headers: rlp.val_at(5)?,
                verify_header_timestamp: rlp.val_at(6)?,
                allow_update_after_misbehavior: rlp.val_at(7)?,
                allow_update_after_expiry: rlp.val_at(8)?,
            })
        }
}

impl ToRlp for StateConfig {
    fn to_rlp(&self) -> Vec<u8> {
        rlp::encode(self)
    }
}

impl FromRlp for StateConfig {
    fn from_rlp(bytes: &[u8]) -> Result<Self, Error> {
        match rlp::decode(&bytes) {
            Ok(config) => Ok(config),
            Err(err) => Err(Kind::RlpDecodeError.context(err).into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct StateEntry {
    pub number: u64, // block number where the snapshot was created
    pub timestamp: u64, // blocks creation time
    pub validators: Vec<Validator>, // set of authorized validators at where the snapshot was created

    // Hash and aggregated seal are required to validate the header against the validator set.
    pub hash: Hash, // block hash where the snapshot was created
    pub aggregated_seal: IstanbulAggregatedSeal, // block aggregated_seal where the snapshot was created
}

impl Encodable for StateEntry {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(5);

        s.append(&self.number);
        s.append(&self.timestamp);

        s.begin_list(self.validators.len());
        for validator in self.validators.iter() {
            s.append(validator);
        }

        s.append(&self.hash.as_ref());
        s.append(&self.aggregated_seal);
    }
}

impl Decodable for StateEntry {
        fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
            let validators: Result<Vec<Validator>, DecoderError> = rlp
                .at(2)?
                .iter()
                .map(|r| {
                    r.as_val()
                })
                .collect();

            Ok(StateEntry{
                validators: validators?,
                number: rlp.val_at(0)?,
                timestamp: rlp.val_at(1)?,
                hash: rlp_list_field_from_bytes(rlp, 3)?,
                aggregated_seal: rlp.val_at(4)?,
            })
        }
}

impl StateEntry {
    pub fn new() -> Self {
        Self {
            number: 0,
            timestamp: 0,
            validators: Vec::new(),
            hash: Hash::default(),
            aggregated_seal: IstanbulAggregatedSeal::new(),
        }
    }

    pub fn verify(&self) -> Result<(), Error> {
        verify_aggregated_seal(
            self.hash,
            &self.validators,
            self.aggregated_seal.clone(),
        )
    }
}

impl ToRlp for StateEntry {
    fn to_rlp(&self) -> Vec<u8> {
        rlp::encode(self)
    }
}

impl FromRlp for StateEntry {
    fn from_rlp(bytes: &[u8]) -> Result<Self, Error> {
        match rlp::decode(&bytes) {
            Ok(header) => Ok(header),
            Err(err) => Err(Kind::RlpDecodeError.context(err).into()),
        }
    }
}
