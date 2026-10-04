pub(crate) struct RodauthHmacSecrets {
    current: Vec<u8>,
    old: Option<Vec<u8>>,
}

impl RodauthHmacSecrets {
    pub(crate) fn current(&self) -> &[u8] {
        &self.current
    }

    pub(crate) fn old(&self) -> Option<&[u8]> {
        self.old.as_deref()
    }
}

pub(crate) fn parse(
    current: Option<String>,
    old: Option<String>,
) -> Result<RodauthHmacSecrets, ()> {
    let current = current.filter(|value| !value.is_empty()).ok_or(())?;
    let old = match old {
        Some(value) if value.is_empty() => return Err(()),
        value => value,
    };
    Ok(RodauthHmacSecrets {
        current: current.into_bytes(),
        old: old.map(String::into_bytes),
    })
}

pub(crate) fn load() -> Result<RodauthHmacSecrets, ()> {
    parse(
        std::env::var("RODAUTH_HMAC_SECRET").ok(),
        std::env::var("RODAUTH_HMAC_OLD_SECRET").ok(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_config_requires_the_rails_secret_and_valid_rotation_input() {
        assert!(parse(None, None).is_err());
        assert!(parse(Some(String::new()), None).is_err());
        assert!(parse(Some("rails-secret".into()), Some(String::new())).is_err());
        let keys = parse(Some("rails-secret".into()), Some("older-secret".into()))
            .expect("configured Rails key pair");
        assert_eq!(keys.current(), b"rails-secret");
        assert_eq!(keys.old(), Some(b"older-secret".as_slice()));
    }
}
