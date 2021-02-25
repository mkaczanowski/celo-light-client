use crate::{Error, Kind, Storage, SerializableStorage};
use rlp::{DecoderError, Decodable, Rlp, Encodable, RlpStream};
use std::collections::HashMap;

pub struct SimpleStorage {
    pub db: HashMap<Vec<u8>, Vec<u8>>,
}

impl SimpleStorage {
    pub fn new(_num_cols: u32) -> Self {
        Self {
            db: HashMap::new()
        }
    }
}

impl<T: Decodable + Encodable> SerializableStorage<T> for SimpleStorage {
    fn serialize(&self) -> Vec<u8> {
        rlp::encode(self)
    }

    fn deserialize(bytes: &[u8]) -> Result<Box<T>, Error> {
       let decoded: T = match rlp::decode(&bytes) {
           Ok(data) => data,
           Err(e) => return Kind::GenericSerializationError.context(e).into(),
       };

       Ok(Box::new(decoded))
    }
}

impl Encodable for SimpleStorage {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(self.db.len());
        for (key, value) in self.db.iter() {
           s.append_list(key);
           s.append_list(value);
        }
    }
}

impl Decodable for SimpleStorage {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let mut map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

        for i in (0..rlp.item_count()?).step_by(2) {
            let key: Vec<u8> = rlp.at(i)?.as_list()?;
            let val: Vec<u8> = rlp.at(i+1)?.as_list()?;

            map.insert(key, val);
        }

        Ok(SimpleStorage {
            db: map
        })
    }
}

impl Storage for SimpleStorage {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        match self.db.insert(key.to_vec(), value.to_vec()) {
            Some(value) => Ok(Some(value.to_vec())),
            None => Ok(None),
        }
    }

    fn get(&self, key: &[u8]) -> Result<Vec<u8>, Error> {
        let result = self.db.get(key);
            match result {
                Some(value) => Ok(value.to_owned()),
                None => Err(Kind::KeyNotFound.into()),
            }
    }

    fn contains_key(&self, key: &[u8]) -> Result<bool, Error> {
       Ok(self.db.contains_key(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_encodes() {
        let mut storage = SimpleStorage::new(&String::from(""));
        storage.put(&vec![1, 1], &vec![11]).unwrap();
        storage.put(&vec![2, 2], &vec![22]).unwrap();

        let serialized: Vec<u8> = rlp::encode(&storage);
        let deserialized: SimpleStorage = rlp::decode(&serialized).unwrap();

        assert_eq!(deserialized.contains_key(&vec![1, 1]).unwrap(), true);
        assert_eq!(deserialized.contains_key(&vec![2, 2]).unwrap(), true);
        assert_eq!(deserialized.contains_key(&vec![3, 3]).unwrap(), false);

        assert_eq!(deserialized.get(&vec![1, 1]).unwrap(), vec![11]);
        assert_eq!(deserialized.get(&vec![2, 2]).unwrap(), vec![22]);
        assert_eq!(deserialized.get(&vec![3, 3]).is_err(), true);
    }
}
