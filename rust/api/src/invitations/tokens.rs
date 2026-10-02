use super::*;

pub(super) fn new_token() -> Result<(String, String), ApiError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| ApiError::internal())?;
    let token = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let digest = format!("{:x}", Sha256::digest(token.as_bytes()));
    Ok((token, digest))
}
