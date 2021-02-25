use crate::types::header::{Address, Hash};
use crate::types::istanbul::SerializedPublicKey;
use crate::traits::{ToRlp, FromRlp};
use crate::serialization::rlp::{rlp_field_from_bytes, rlp_list_field_from_bytes};
use crate::errors::{Error, Kind};

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
pub struct StateEntry {
    pub validators: Vec<Validator>, // set of authorized validators at this moment
    pub epoch: u64, // the number of blocks for each epoch
    pub number: u64, // block number where the snapshot was created
    pub hash: Hash, // block hash where the snapshot was created
}

impl Encodable for StateEntry {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(4);

        s.begin_list(self.validators.len());
        for validator in self.validators.iter() {
            s.append(validator);
        }

        s.append(&self.epoch);
        s.append(&self.number);
        s.append(&self.hash.as_ref());
    }
}

impl Decodable for StateEntry {
        fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
            let validators: Result<Vec<Validator>, DecoderError> = rlp
                .at(0)?
                .iter()
                .map(|r| {
                    r.as_val()
                })
                .collect();

            Ok(StateEntry{
                validators: validators?,
                epoch: rlp.val_at(1)?,
                number: rlp.val_at(2)?,
                hash: rlp_list_field_from_bytes(rlp, 3)?,
            })
        }
}

impl StateEntry {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
            epoch: 0,
            number: 0,
            hash: Hash::default()
        }
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
