use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use oxide_auth::code_grant::extensions::Pkce;

pub(super) fn valid_challenge(method: &str, challenge: &str) -> bool {
    method == "S256"
        && URL_SAFE_NO_PAD
            .decode(challenge)
            .is_ok_and(|bytes| bytes.len() == 32)
}

pub(super) fn verify(method: Option<&str>, challenge: Option<&str>, verifier: &str) -> bool {
    if !(43..=128).contains(&verifier.len())
        || !verifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(&byte))
    {
        return false;
    }
    let (Some(method), Some(challenge)) = (method, challenge) else {
        return false;
    };
    if !valid_challenge(method, challenge) {
        return false;
    }
    let pkce = Pkce::required();
    let Ok(state) = pkce.challenge(Some(method.into()), Some(challenge.into())) else {
        return false;
    };
    pkce.verify(state, Some(verifier.into())).is_ok()
}

#[cfg(test)]
mod tests {
    use super::verify;

    #[test]
    fn rfc7636_vector_and_negative_verifiers() {
        let challenge = Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert!(verify(Some("S256"), challenge, verifier));
        for invalid in [
            "",
            "short",
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXl",
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjX!",
        ] {
            assert!(!verify(Some("S256"), challenge, invalid));
        }
        assert!(!verify(Some("plain"), Some(verifier), verifier));
        assert!(!verify(None, challenge, verifier));
        assert!(!verify(Some("S256"), None, verifier));
    }
}
