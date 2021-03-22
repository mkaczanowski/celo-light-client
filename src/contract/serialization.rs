use crate::traits::FromRlp;
use cosmwasm_std::StdError;

pub fn from_base64<T>(base64_data: &String, target_type: String) -> Result<T, StdError> where T: FromRlp {
    let bytes = match base64::decode(base64_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err(StdError::ParseErr {
                target_type,
                msg: format!("Unable to base64 decode data. Error: {}", e),
            })
        }
    };

    match T::from_rlp(bytes.as_slice()) {
        Ok(block) => return Ok(block),
        Err(e) => {
            return Err(StdError::ParseErr {
                target_type,
                msg: format!("Unable to rlp decode from base64 data. Error: {}", e),
            })
        }
    };
}
