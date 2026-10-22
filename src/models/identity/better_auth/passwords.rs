use std::{collections::HashSet, sync::LazyLock};

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
