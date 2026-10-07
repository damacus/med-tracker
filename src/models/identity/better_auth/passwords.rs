use std::{collections::HashSet, sync::LazyLock};
use async_trait::async_trait;
use better_auth_core::{AuthError, AuthResult, utils::password::{PasswordHasher, hash_password, verify_password}};

pub(super) struct CompatibleHasher;

#[async_trait]
impl PasswordHasher for CompatibleHasher {
    async fn hash(&self, password: &str) -> AuthResult<String> { hash_password(None, password).await }

    async fn verify(&self, hash: &str, password: &str) -> AuthResult<bool> {
        if hash.starts_with("$2") {
            let hash = hash.to_owned();
            let password = password.to_owned();
            return tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash)).await.map_err(|_| AuthError::internal("Password verification unavailable"))?.map_err(|_| AuthError::InvalidCredentials);
        }
        match verify_password(None, password, hash).await {
            Ok(()) => Ok(true),
            Err(AuthError::InvalidCredentials) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

static BLOCKED: LazyLock<HashSet<String>> = LazyLock::new(|| {
    include_str!("../../../../assets/security/common-passwords.txt")
        .lines()
        .map(str::to_lowercase)
        .collect()
});

pub fn validate_new_password(password: &str) -> Result<(), &'static str> {
    if !(15..=1024).contains(&password.chars().count()) {
        return Err("Password must contain between 15 and 1024 characters.");
    }
    if BLOCKED.contains(&password.to_lowercase()) {
        return Err("Choose a password that is not commonly used or breached.");
    }
    Ok(())
}
