use med_tracker::models::identity::totp::{
    OtpCompatibilityError, SecretVersion, derive_rodauth_otp_secret, verify_rodauth_otp,
};

const CURRENT_SECRET: &[u8] = b"synthetic-rails-secret-key-base-for-compatibility";
const OLD_SECRET: &[u8] = b"synthetic-old-rails-secret-key-base";
const SHORT_SEED: &str = "abcdefghijklmnop";
const LONG_SEED: &str = "abcdefghijklmnopqrstuvwxyz234567";
const NOW: i64 = 1_700_000_000;

#[test]
fn legacy_hmac_derivation_matches_locked_rodauth_vectors() {
    for (seed, secret, expected) in [
        (SHORT_SEED, CURRENT_SECRET, "4tlxacqrzvpp3asc"),
        (SHORT_SEED, OLD_SECRET, "wdhelz44632d4ntv"),
        (
            LONG_SEED,
            CURRENT_SECRET,
            "ugpjb54nzdldum6kwhirumb2j65ga6fl",
        ),
        (LONG_SEED, OLD_SECRET, "r6nuz5lg2oracz6lm6c7do2pxgksubxa"),
    ] {
        assert_eq!(
            derive_rodauth_otp_secret(seed, Some(secret)),
            Ok(expected.into())
        );
    }
}

#[test]
fn explicitly_disabled_hmac_preserves_existing_stored_secret() {
    assert_eq!(
        derive_rodauth_otp_secret(SHORT_SEED, None),
        Ok(SHORT_SEED.into())
    );
    assert_eq!(
        derive_rodauth_otp_secret(LONG_SEED, None),
        Ok(LONG_SEED.into())
    );
}

#[test]
fn hmac_disabled_codes_match_rotp_and_ignore_old_secret_configuration() {
    for (seed, code) in [(SHORT_SEED, "541083"), (LONG_SEED, "532659")] {
        let result = verify_rodauth_otp(seed, None, Some(OLD_SECRET), code, NOW, NOW - 61)
            .unwrap()
            .expect("raw seed accepted only when HMAC is explicitly disabled");
        assert_eq!(result.secret_version, SecretVersion::Current);
        assert_eq!(result.matched_step, (NOW / 30) as u64);
    }
}

#[test]
fn invalid_legacy_seed_and_empty_configured_hmac_secret_fail_closed() {
    for seed in [
        "",
        "abc",
        "ABCDEFGHIJKLMNOP",
        "abcdefghijklmnop=",
        "abcdefghijklmno1",
    ] {
        assert_eq!(
            derive_rodauth_otp_secret(seed, Some(CURRENT_SECRET)),
            Err(OtpCompatibilityError::InvalidStoredKey)
        );
        assert_eq!(
            derive_rodauth_otp_secret(seed, None),
            Err(OtpCompatibilityError::InvalidStoredKey)
        );
    }
    assert_eq!(
        derive_rodauth_otp_secret(SHORT_SEED, Some(b"")),
        Err(OtpCompatibilityError::EmptyHmacSecret)
    );
}

#[test]
fn fixed_time_codes_match_locked_rotp_and_report_the_matched_step() {
    for (seed, code) in [(SHORT_SEED, "649638"), (LONG_SEED, "055041")] {
        let result = verify_rodauth_otp(seed, Some(CURRENT_SECRET), None, code, NOW, NOW - 61)
            .expect("valid compatibility inputs")
            .expect("Rails code authenticates");
        assert_eq!(result.matched_step, (NOW / 30) as u64);
        assert_eq!(result.secret_version, SecretVersion::Current);
    }
}

#[test]
fn drift_accepts_one_step_either_side_and_rejects_two_steps() {
    for (code, offset) in [("435136", -30), ("649638", 0), ("730930", 30)] {
        let result =
            verify_rodauth_otp(SHORT_SEED, Some(CURRENT_SECRET), None, code, NOW, NOW - 61)
                .unwrap()
                .expect("code within Rails drift");
        assert_eq!(result.matched_step, ((NOW + offset) / 30) as u64);
    }
    for code in ["269065", "987293", "000000"] {
        assert_eq!(
            verify_rodauth_otp(SHORT_SEED, Some(CURRENT_SECRET), None, code, NOW, NOW - 61),
            Ok(None)
        );
    }
}

#[test]
fn a_code_collision_reports_the_latest_matching_step_like_rotp() {
    let now = 1_709_698_170;
    let result = verify_rodauth_otp(
        SHORT_SEED,
        Some(CURRENT_SECRET),
        None,
        "975469",
        now,
        now - 91,
    )
    .unwrap()
    .expect("synthetic adjacent-step collision authenticates");
    assert_eq!(result.matched_step, 56_989_940);
}

#[test]
fn interval_gate_is_strictly_more_than_thirty_seconds_since_last_use() {
    for seconds_since_last_use in [0, 29, 30] {
        assert_eq!(
            verify_rodauth_otp(
                SHORT_SEED,
                Some(CURRENT_SECRET),
                None,
                "649638",
                NOW,
                NOW - seconds_since_last_use
            ),
            Ok(None)
        );
    }
    assert!(
        verify_rodauth_otp(
            SHORT_SEED,
            Some(CURRENT_SECRET),
            None,
            "649638",
            NOW,
            NOW - 31
        )
        .unwrap()
        .is_some()
    );
}

#[test]
fn a_step_not_strictly_after_last_use_is_rejected_even_when_interval_passes() {
    assert_eq!(
        verify_rodauth_otp(
            SHORT_SEED,
            Some(CURRENT_SECRET),
            None,
            "435136",
            NOW,
            NOW - 31
        ),
        Ok(None)
    );
    assert!(
        verify_rodauth_otp(
            SHORT_SEED,
            Some(CURRENT_SECRET),
            None,
            "435136",
            NOW,
            NOW - 61
        )
        .unwrap()
        .is_some()
    );
}

#[test]
fn old_secret_rotation_is_explicit_and_uses_the_same_replay_gate() {
    for (seed, code) in [(SHORT_SEED, "621095"), (LONG_SEED, "973973")] {
        assert_eq!(
            verify_rodauth_otp(seed, Some(CURRENT_SECRET), None, code, NOW, NOW - 61),
            Ok(None)
        );
        let result = verify_rodauth_otp(
            seed,
            Some(CURRENT_SECRET),
            Some(OLD_SECRET),
            code,
            NOW,
            NOW - 61,
        )
        .unwrap()
        .expect("configured old secret accepted");
        assert_eq!(result.secret_version, SecretVersion::Old);
        assert_eq!(result.matched_step, (NOW / 30) as u64);
        assert_eq!(
            verify_rodauth_otp(
                seed,
                Some(CURRENT_SECRET),
                Some(OLD_SECRET),
                code,
                NOW,
                NOW - 30
            ),
            Ok(None)
        );
    }
}

#[test]
fn code_normalisation_matches_ascii_whitespace_handling_without_unicode_aliases() {
    for code in [" 649\t638\n", "649\u{b}638", "649\u{c}638\r"] {
        assert!(
            verify_rodauth_otp(SHORT_SEED, Some(CURRENT_SECRET), None, code, NOW, NOW - 61)
                .unwrap()
                .is_some()
        );
    }
    for code in ["649\u{a0}638", "６４９６３８", "6496380", "abc638"] {
        assert_eq!(
            verify_rodauth_otp(SHORT_SEED, Some(CURRENT_SECRET), None, code, NOW, NOW - 61),
            Ok(None)
        );
    }
}

#[test]
fn impossible_timestamps_fail_closed_without_wrapping() {
    assert_eq!(
        verify_rodauth_otp(SHORT_SEED, Some(CURRENT_SECRET), None, "649638", -1, 0),
        Err(OtpCompatibilityError::InvalidTimestamp)
    );
    assert_eq!(
        verify_rodauth_otp(SHORT_SEED, Some(CURRENT_SECRET), None, "649638", NOW, -1),
        Err(OtpCompatibilityError::InvalidTimestamp)
    );
    assert_eq!(
        verify_rodauth_otp(
            SHORT_SEED,
            Some(CURRENT_SECRET),
            None,
            "649638",
            NOW,
            i64::MAX
        ),
        Ok(None)
    );
}
