use crate::errors::Error;

// "Deafult" trait is implemented for a few selected fixed-array types. Taken we can't implement
// the trait outside of a crate, we created a new one that mimics the stdlib.
pub trait DefaultFrom {
    fn default() -> Self;
}

pub trait FromBytes {
    fn from_bytes(data: &[u8]) -> Result<&Self, Error>;
}

pub trait ToRlp {
    fn to_rlp(&self) -> Vec<u8>;
}

pub trait FromRlp {
    fn from_rlp(bytes: &[u8]) -> Result<Self, Error>
    where
        Self: std::marker::Sized;
}

pub trait StateConfig {
    fn epoch_size(&self) -> u64;
    fn allowed_clock_skew(&self) -> u64;

    fn verify_epoch_headers(&self) -> bool;
    fn verify_non_epoch_headers(&self) -> bool;
    fn verify_header_timestamp(&self) -> bool;
}
