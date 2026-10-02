use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use medtracker_api::webauthn::{verify_passkey_assertion, PasskeyVerifyError};
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use sha2::{Digest, Sha256};

const ORIGIN: &str = "http://localhost:39998";
const RP_ID: &str = "localhost";
const CHALLENGE: &str = "dGVzdC1jaGFsbGVuZ2UtYnRlcy1hcmUtc3VmZmljaWVudA";

fn signing_key() -> SigningKey {
    SigningKey::from_slice(&[0x2Au8; 32]).expect("fixed test key")
}

fn cose_ec2_key(x: &[u8], y: &[u8]) -> Vec<u8> {
    let mut key = vec![0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20];
    key.extend_from_slice(x);
    key.push(0x22);
    key.push(0x58);
    key.push(0x20);
    key.extend_from_slice(y);
    key
}

fn stored_key_b64() -> String {
    let point = signing_key().verifying_key().to_sec1_point(false);
    URL_SAFE_NO_PAD.encode(cose_ec2_key(point.x().unwrap(), point.y().unwrap()))
}

fn auth_data(flags: u8, sign_count: u32) -> Vec<u8> {
    let mut data = Sha256::digest(RP_ID.as_bytes()).to_vec();
    data.push(flags);
    data.extend_from_slice(&sign_count.to_be_bytes());
    data
}

fn client_data(challenge: &str, origin: &str, credential_type: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": credential_type,
        "challenge": challenge,
        "origin": origin,
        "crossOrigin": false,
    }))
    .unwrap()
}

fn signed_payload(auth: &[u8], client: &[u8], user_handle: Option<&[u8]>) -> String {
    let mut signed = auth.to_vec();
    signed.extend_from_slice(&Sha256::digest(client));
    let signature: Signature = signing_key().sign(&signed);
    let credential_id = URL_SAFE_NO_PAD.encode([0x11u8; 32]);
    let mut response = serde_json::json!({
        "authenticatorData": URL_SAFE_NO_PAD.encode(auth),
        "clientDataJSON": URL_SAFE_NO_PAD.encode(client),
        "signature": URL_SAFE_NO_PAD.encode(signature.to_der().as_bytes()),
    });
    if let Some(handle) = user_handle {
        response["userHandle"] = serde_json::json!(URL_SAFE_NO_PAD.encode(handle));
    }
    serde_json::json!({
        "id": credential_id,
        "rawId": credential_id,
        "type": "public-key",
        "response": response,
    })
    .to_string()
}

fn valid_payload(flags: u8, sign_count: u32) -> String {
    signed_payload(
        &auth_data(flags, sign_count),
        &client_data(CHALLENGE, ORIGIN, "webauthn.get"),
        Some(b"user-handle-32-bytes-of-entropy-1234"),
    )
}

#[test]
fn a_valid_assertion_returns_the_authenticator_sign_count_and_user_handle() {
    let verified = verify_passkey_assertion(
        &valid_payload(0x05, 7),
        CHALLENGE,
        ORIGIN,
        RP_ID,
        &stored_key_b64(),
        6,
    )
    .expect("valid assertion");
    assert_eq!(verified.sign_count, 7);
    assert_eq!(
        verified.user_handle.as_deref(),
        Some(b"user-handle-32-bytes-of-entropy-1234".as_slice())
    );
}

#[test]
fn a_zero_counter_pair_is_accepted_for_authenticators_without_counters() {
    let verified = verify_passkey_assertion(
        &valid_payload(0x05, 0),
        CHALLENGE,
        ORIGIN,
        RP_ID,
        &stored_key_b64(),
        0,
    )
    .expect("counterless authenticator");
    assert_eq!(verified.sign_count, 0);
}

#[test]
fn a_non_increasing_counter_is_rejected_as_a_cloned_authenticator() {
    for (stored, presented) in [(7_u64, 7_u32), (8, 7)] {
        assert_eq!(
            verify_passkey_assertion(
                &valid_payload(0x05, presented),
                CHALLENGE,
                ORIGIN,
                RP_ID,
                &stored_key_b64(),
                stored,
            ),
            Err(PasskeyVerifyError::SignCountReplay)
        );
    }
}

#[test]
fn a_zero_presented_counter_is_rejected_when_a_positive_counter_is_stored() {
    assert_eq!(
        verify_passkey_assertion(
            &valid_payload(0x05, 0),
            CHALLENGE,
            ORIGIN,
            RP_ID,
            &stored_key_b64(),
            3,
        ),
        Err(PasskeyVerifyError::SignCountReplay)
    );
}

#[test]
fn a_different_challenge_is_rejected() {
    let payload = signed_payload(
        &auth_data(0x05, 1),
        &client_data("b3RoZXItY2hhbGxlbmdlLXZhbHVlLXh4eA", ORIGIN, "webauthn.get"),
        None,
    );
    assert_eq!(
        verify_passkey_assertion(&payload, CHALLENGE, ORIGIN, RP_ID, &stored_key_b64(), 0),
        Err(PasskeyVerifyError::ChallengeMismatch)
    );
}

#[test]
fn a_different_origin_is_rejected() {
    let payload = signed_payload(
        &auth_data(0x05, 1),
        &client_data(CHALLENGE, "https://evil.example", "webauthn.get"),
        None,
    );
    assert_eq!(
        verify_passkey_assertion(&payload, CHALLENGE, ORIGIN, RP_ID, &stored_key_b64(), 0),
        Err(PasskeyVerifyError::OriginMismatch)
    );
}

#[test]
fn an_attestation_response_type_is_rejected() {
    let payload = signed_payload(
        &auth_data(0x05, 1),
        &client_data(CHALLENGE, ORIGIN, "webauthn.create"),
        None,
    );
    assert_eq!(
        verify_passkey_assertion(&payload, CHALLENGE, ORIGIN, RP_ID, &stored_key_b64(), 0),
        Err(PasskeyVerifyError::UnexpectedType)
    );
}

#[test]
fn a_different_relying_party_is_rejected() {
    let payload = valid_payload(0x05, 1);
    assert_eq!(
        verify_passkey_assertion(
            &payload,
            CHALLENGE,
            ORIGIN,
            "example.com",
            &stored_key_b64(),
            0
        ),
        Err(PasskeyVerifyError::RpIdMismatch)
    );
}

#[test]
fn user_presence_and_verification_flags_are_required() {
    for flags in [0x00_u8, 0x01, 0x04] {
        assert_eq!(
            verify_passkey_assertion(
                &valid_payload(flags, 1),
                CHALLENGE,
                ORIGIN,
                RP_ID,
                &stored_key_b64(),
                0,
            ),
            Err(PasskeyVerifyError::MissingUserFlags)
        );
    }
}

#[test]
fn a_tampered_signature_is_rejected() {
    let auth = auth_data(0x05, 1);
    let client = client_data(CHALLENGE, ORIGIN, "webauthn.get");
    let wrong_signature: Signature = signing_key().sign(b"different message bytes");
    let credential_id = URL_SAFE_NO_PAD.encode([0x11u8; 32]);
    let payload = serde_json::json!({
        "id": credential_id,
        "rawId": credential_id,
        "type": "public-key",
        "response": {
            "authenticatorData": URL_SAFE_NO_PAD.encode(&auth),
            "clientDataJSON": URL_SAFE_NO_PAD.encode(&client),
            "signature": URL_SAFE_NO_PAD.encode(wrong_signature.to_der().as_bytes()),
        },
    })
    .to_string();
    assert_eq!(
        verify_passkey_assertion(&payload, CHALLENGE, ORIGIN, RP_ID, &stored_key_b64(), 0),
        Err(PasskeyVerifyError::SignatureInvalid)
    );
}

#[test]
fn a_signature_from_a_different_key_is_rejected() {
    let wrong_key = SigningKey::from_slice(&[0x35u8; 32]).unwrap();
    let point = wrong_key.verifying_key().to_sec1_point(false);
    let stored = URL_SAFE_NO_PAD.encode(cose_ec2_key(point.x().unwrap(), point.y().unwrap()));
    assert_eq!(
        verify_passkey_assertion(
            &valid_payload(0x05, 1),
            CHALLENGE,
            ORIGIN,
            RP_ID,
            &stored,
            0
        ),
        Err(PasskeyVerifyError::SignatureInvalid)
    );
}

#[test]
fn malformed_payloads_are_rejected() {
    for payload in [
        "",
        "not json",
        "{}",
        "{\"id\":1}",
        "{\"id\":\"x\",\"response\":{}}",
    ] {
        assert_eq!(
            verify_passkey_assertion(payload, CHALLENGE, ORIGIN, RP_ID, &stored_key_b64(), 0),
            Err(PasskeyVerifyError::Malformed)
        );
    }
}

#[test]
fn a_malformed_stored_key_is_rejected() {
    for stored in ["", "%%%", "aGVsbG8"] {
        assert_eq!(
            verify_passkey_assertion(&valid_payload(0x05, 1), CHALLENGE, ORIGIN, RP_ID, stored, 0),
            Err(PasskeyVerifyError::UnsupportedKey)
        );
    }
}
