use super::*;
use sha2::{Digest, Sha256};

pub(super) fn issue() -> Result<(String, String), OperationError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| OperationError::Unavailable)?;
    let token = hex::encode(bytes);
    Ok((token.clone(), digest(&token)))
}

pub(super) fn digest(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
