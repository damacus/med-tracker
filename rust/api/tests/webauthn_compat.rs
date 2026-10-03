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

#[test]
fn cross_origin_assertions_are_rejected() {
    let client = serde_json::to_vec(&serde_json::json!({
        "type": "webauthn.get", "challenge": CHALLENGE,
        "origin": ORIGIN, "crossOrigin": true, "topOrigin": "https://evil.example"
    }))
    .unwrap();
    let payload = signed_payload(&auth_data(0x05, 1), &client, None);
    assert!(
        verify_passkey_assertion(&payload, CHALLENGE, ORIGIN, RP_ID, &stored_key_b64(), 0).is_err()
    );
}

#[test]
fn inconsistent_credential_identifiers_and_types_are_rejected() {
    for (field, value) in [("rawId", "b3RoZXI"), ("type", "password")] {
        let mut payload: serde_json::Value = serde_json::from_str(&valid_payload(0x05, 1)).unwrap();
        payload[field] = value.into();
        assert!(verify_passkey_assertion(
            &payload.to_string(),
            CHALLENGE,
            ORIGIN,
            RP_ID,
            &stored_key_b64(),
            0
        )
        .is_err());
    }
}

#[test]
fn backup_state_requires_backup_eligibility() {
    assert!(verify_passkey_assertion(
        &valid_payload(0x15, 1),
        CHALLENGE,
        ORIGIN,
        RP_ID,
        &stored_key_b64(),
        0
    )
    .is_err());
}

#[test]
fn library_algorithms_verify_and_unsupported_algorithms_fail() {
    use ciborium::value::Value as Cbor;
    use openssl::{
        bn::{BigNum, BigNumContext},
        ec::{EcGroup, EcKey},
        hash::MessageDigest,
        nid::Nid,
        pkey::PKey,
        rsa::{Padding, Rsa},
        sign::{RsaPssSaltlen, Signer},
    };
    for (algorithm, ed448) in [-7_i64, -35, -8, -257, -258, -259, -37, -38, -39]
        .map(|algorithm| (algorithm, false))
        .into_iter()
        .chain([(-8, true)])
    {
        let mut entries = vec![(Cbor::Integer(3.into()), Cbor::Integer(algorithm.into()))];
        let key = match algorithm {
            -7 | -35 => {
                let (curve, nid, size) = if algorithm == -7 {
                    (1, Nid::X9_62_PRIME256V1, 32)
                } else {
                    (2, Nid::SECP384R1, 48)
                };
                let group = EcGroup::from_curve_name(nid).unwrap();
                let key = EcKey::generate(&group).unwrap();
                let mut x = BigNum::new().unwrap();
                let mut y = BigNum::new().unwrap();
                key.public_key()
                    .affine_coordinates_gfp(
                        &group,
                        &mut x,
                        &mut y,
                        &mut BigNumContext::new().unwrap(),
                    )
                    .unwrap();
                entries.extend(
                    [
                        (1, Cbor::Integer(2.into())),
                        (-1, Cbor::Integer(curve.into())),
                        (-2, Cbor::Bytes(x.to_vec_padded(size).unwrap())),
                        (-3, Cbor::Bytes(y.to_vec_padded(size).unwrap())),
                    ]
                    .map(|(label, value)| (Cbor::Integer(label.into()), value)),
                );
                PKey::from_ec_key(key).unwrap()
            }
            -8 => {
                let key = if ed448 {
                    PKey::generate_ed448().unwrap()
                } else {
                    PKey::generate_ed25519().unwrap()
                };
                entries.extend(
                    [
                        (1, Cbor::Integer(1.into())),
                        (-1, Cbor::Integer(if ed448 { 7.into() } else { 6.into() })),
                        (-2, Cbor::Bytes(key.raw_public_key().unwrap())),
                    ]
                    .map(|(label, value)| (Cbor::Integer(label.into()), value)),
                );
                key
            }
            _ => {
                let key = Rsa::generate(2048).unwrap();
                entries.extend(
                    [
                        (1, Cbor::Integer(3.into())),
                        (-1, Cbor::Bytes(key.n().to_vec())),
                        (-2, Cbor::Bytes(key.e().to_vec())),
                    ]
                    .map(|(label, value)| (Cbor::Integer(label.into()), value)),
                );
                PKey::from_rsa(key).unwrap()
            }
        };
        let mut encoded_key = Vec::new();
        ciborium::ser::into_writer(&Cbor::Map(entries), &mut encoded_key).unwrap();
        let stored = URL_SAFE_NO_PAD.encode(encoded_key);
        let auth = auth_data(5, 1);
        let client = client_data(CHALLENGE, ORIGIN, "webauthn.get");
        let mut message = auth.clone();
        message.extend_from_slice(&Sha256::digest(&client));
        let digest = match algorithm {
            -35 | -258 | -38 => MessageDigest::sha384(),
            -259 | -39 => MessageDigest::sha512(),
            _ => MessageDigest::sha256(),
        };
        let mut signer = if algorithm == -8 {
            Signer::new_without_digest(&key).unwrap()
        } else {
            Signer::new(digest, &key).unwrap()
        };
        if [-37, -38, -39].contains(&algorithm) {
            signer.set_rsa_padding(Padding::PKCS1_PSS).unwrap();
            signer
                .set_rsa_pss_saltlen(RsaPssSaltlen::DIGEST_LENGTH)
                .unwrap();
            signer.set_rsa_mgf1_md(digest).unwrap();
        }
        let mut signature = signer.sign_oneshot_to_vec(&message).unwrap();
        for tampered in [false, true] {
            if tampered {
                signature[0] ^= 1;
            }
            let id = URL_SAFE_NO_PAD.encode([17; 32]);
            let payload = serde_json::json!({"id": id, "rawId": id, "type": "public-key", "response": {
                "authenticatorData": URL_SAFE_NO_PAD.encode(&auth), "clientDataJSON": URL_SAFE_NO_PAD.encode(&client), "signature": URL_SAFE_NO_PAD.encode(&signature)
            }}).to_string();
            let result = verify_passkey_assertion(&payload, CHALLENGE, ORIGIN, RP_ID, &stored, 0);
            assert_eq!(
                result.is_ok(),
                !tampered && [-7, -8, -257].contains(&algorithm),
                "algorithm {algorithm}, ed448 {ed448}: {result:?}"
            );
        }
    }
}

#[test]
fn origins_with_paths_and_credentials_are_not_valid_webauthn_origins() {
    for origin in [
        "http://localhost:39998/path",
        "http://user@localhost:39998",
        "http://localhost:39998/?query=yes",
    ] {
        let payload = signed_payload(
            &auth_data(5, 1),
            &client_data(CHALLENGE, origin, "webauthn.get"),
            None,
        );
        assert!(
            verify_passkey_assertion(&payload, CHALLENGE, ORIGIN, RP_ID, &stored_key_b64(), 0)
                .is_err()
        );
    }
}
