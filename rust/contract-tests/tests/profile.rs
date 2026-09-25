use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::{multipart, Response};
use serde_json::{json, Value};

fn profile_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/profile",
        fixture.profile_household_id
    )
}

fn avatar_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/profile/avatar",
        fixture.avatar_household_id
    )
}

fn data(response: Response) -> Value {
    response.json::<Value>().expect("JSON response")["data"].clone()
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn assert_no_store(response: &Response) {
    assert!(response.headers()["cache-control"]
        .to_str()
        .expect("cache control")
        .contains("no-store"));
}

fn assert_error(response: Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let id = request_id(&response);
    let body: Value = response.json().expect("JSON error");
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["request_id"], id);
    assert!(body.get("data").is_none());
    body
}

fn avatar_form(bytes: Vec<u8>, filename: &str, mime: &str) -> multipart::Form {
    let part = multipart::Part::bytes(bytes)
        .file_name(filename.to_owned())
        .mime_str(mime)
        .expect("valid MIME");
    multipart::Form::new().part("avatar", part)
}

fn download(target: &Target, path: &str, token: &str, mime: &str, filename: &str, bytes: &[u8]) {
    let response = target.get(path, Some(token));
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    assert_eq!(response.headers()["content-type"], mime);
    assert!(response.headers()["content-disposition"]
        .to_str()
        .expect("content disposition")
        .contains(filename));
    assert_eq!(response.bytes().expect("avatar bytes").as_ref(), bytes);
}

fn audit_rows(target: &Target, fixture: &Fixture) -> Vec<Value> {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.avatar_household_id
    );
    let response = target.get(&path, Some(&fixture.avatar_access_token));
    assert_eq!(response.status().as_u16(), 200);
    data(response).as_array().expect("audit rows").clone()
}

#[test]
fn profile_get_patch_put_and_invalid_update_preserve_public_state() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = profile_path(&fixture);
    let response = target.get(&path, Some(&fixture.profile_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    let original = data(response);
    assert_eq!(
        original["account_id"],
        fixture.profile_account_id.to_string()
    );
    assert_eq!(original["person_id"], fixture.profile_person_id.to_string());
    assert!(original["date_of_birth"].is_string());
    assert!(original["time_zone"].is_string());
    assert!(original["gravatar_enabled"].is_boolean());
    assert!(original["mobile_shortcuts"].is_array());
    assert_eq!(original["avatar_attached"], false);

    let response = target.patch_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {"date_of_birth": "1985-06-15", "time_zone": "Europe/London",
            "gravatar_enabled": true, "mobile_shortcuts": ["profile", "finder"]}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    let patched = data(response);
    assert_eq!(patched["date_of_birth"], "1985-06-15");
    assert_eq!(patched["time_zone"], "Europe/London");
    assert_eq!(patched["gravatar_enabled"], true);
    assert_eq!(patched["mobile_shortcuts"], json!(["profile", "finder"]));
    assert_eq!(
        data(target.get(&path, Some(&fixture.profile_access_token))),
        patched
    );

    let response = target.put_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {"date_of_birth": "1984-07-16", "time_zone": "Europe/Paris",
            "gravatar_enabled": false, "mobile_shortcuts": ["inventory"]}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    let saved = data(response);
    assert_eq!(saved["date_of_birth"], "1984-07-16");
    assert_eq!(saved["time_zone"], "Europe/Paris");
    assert_eq!(saved["gravatar_enabled"], false);
    assert_eq!(saved["mobile_shortcuts"], json!(["inventory"]));
    assert_eq!(
        data(target.get(&path, Some(&fixture.profile_access_token))),
        saved
    );

    let response = target.put_json(
        &path,
        &fixture.profile_access_token,
        &json!({"profile": {"time_zone": "UTC"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let retained = data(response);
    assert_eq!(retained["date_of_birth"], saved["date_of_birth"]);
    assert_eq!(retained["gravatar_enabled"], saved["gravatar_enabled"]);
    assert_eq!(retained["mobile_shortcuts"], saved["mobile_shortcuts"]);
    assert_eq!(retained["time_zone"], "UTC");
    assert_eq!(
        data(target.get(&path, Some(&fixture.profile_access_token))),
        retained
    );
    assert_error(
        target.put_json(
            &path,
            &fixture.profile_access_token,
            &json!({"profile": {"date_of_birth": "1981-01-01", "time_zone": "Invalid/Place"}}),
        ),
        422,
        "validation_failed",
    );
    assert_eq!(
        data(target.get(&path, Some(&fixture.profile_access_token))),
        retained
    );

    for (invalid, code) in [
        (
            json!({"date_of_birth": "1981-01-01", "time_zone": "Invalid/Place"}),
            "validation_failed",
        ),
        (
            json!({"date_of_birth": "not-a-date", "time_zone": "Europe/London"}),
            "unprocessable_content",
        ),
        (
            json!({"mobile_shortcuts": ["profile", "profile"], "time_zone": "Europe/London"}),
            "validation_failed",
        ),
        (
            json!({"email": "attacker@example.test", "time_zone": "Europe/London"}),
            "unprocessable_content",
        ),
    ] {
        assert_error(
            target.patch_json(
                &path,
                &fixture.profile_access_token,
                &json!({"profile": invalid}),
            ),
            422,
            code,
        );
        assert_eq!(
            data(target.get(&path, Some(&fixture.profile_access_token))),
            retained
        );
    }
}

#[test]
fn profile_access_is_self_scoped_and_checks_current_grants() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = profile_path(&fixture);
    assert_error(target.get(&path, None), 401, "unauthorized");
    assert_error(
        target.get(&path, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    let foreign_path = format!(
        "/api/v1/households/{}/profile",
        fixture.foreign_household_id
    );
    let foreign = assert_error(
        target.get(&foreign_path, Some(&fixture.profile_access_token)),
        403,
        "forbidden",
    );
    assert!(!foreign.to_string().contains(&fixture.foreign_email));
    let response = target.get(&path, Some(&fixture.profile_view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let viewed = data(response);
    assert_eq!(
        viewed["account_id"],
        fixture.profile_view_account_id.to_string()
    );
    assert_eq!(
        viewed["person_id"],
        fixture.profile_view_person_id.to_string()
    );
    assert_ne!(viewed["person_id"], fixture.profile_person_id.to_string());
    assert_error(
        target.patch_json(
            &path,
            &fixture.profile_view_access_token,
            &json!({"profile": {"time_zone": "UTC"}}),
        ),
        403,
        "forbidden",
    );
    let self_avatar = format!("{path}/avatar");
    assert_error(
        target.get(&self_avatar, Some(&fixture.profile_view_access_token)),
        404,
        "not_found",
    );
    assert_error(
        target.put_multipart(
            &self_avatar,
            &fixture.profile_view_access_token,
            avatar_form(b"view-only".to_vec(), "view.png", "image/png"),
        ),
        403,
        "forbidden",
    );
    assert_error(
        target.delete(&self_avatar, Some(&fixture.profile_view_access_token)),
        403,
        "forbidden",
    );

    let response = target.get(&path, Some(&fixture.profile_revoke_mobile_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        data(response)["person_id"],
        fixture.profile_revoke_person_id.to_string()
    );
    let private_avatar = b"revoked-person-private-avatar\0bytes".to_vec();
    let response = target.put_multipart(
        &self_avatar,
        &fixture.profile_revoke_mobile_token,
        avatar_form(private_avatar.clone(), "private.png", "image/png"),
    );
    assert_eq!(response.status().as_u16(), 200);
    download(
        &target,
        &self_avatar,
        &fixture.profile_revoke_mobile_token,
        "image/png",
        "private.png",
        &private_avatar,
    );
    let grants = format!(
        "/api/v1/households/{}/admin/person_access_grants/{}",
        fixture.profile_household_id, fixture.profile_revoke_grant_id
    );
    assert_eq!(
        target
            .delete(&grants, Some(&fixture.profile_access_token))
            .status()
            .as_u16(),
        204
    );
    assert_error(
        target.get(&path, Some(&fixture.profile_revoke_mobile_token)),
        403,
        "forbidden",
    );
    assert_error(
        target.patch_json(
            &path,
            &fixture.profile_revoke_mobile_token,
            &json!({"profile": {"time_zone": "UTC"}}),
        ),
        403,
        "forbidden",
    );
    assert_error(
        target.get(&self_avatar, Some(&fixture.profile_revoke_mobile_token)),
        403,
        "forbidden",
    );
    assert_error(
        target.put_multipart(
            &self_avatar,
            &fixture.profile_revoke_mobile_token,
            avatar_form(b"revoked".to_vec(), "revoked.png", "image/png"),
        ),
        403,
        "forbidden",
    );
    assert_error(
        target.delete(&self_avatar, Some(&fixture.profile_revoke_mobile_token)),
        403,
        "forbidden",
    );
    assert_error(
        target.get(&path, Some(&fixture.profile_revoke_access_token)),
        401,
        "unauthorized",
    );
}

#[test]
fn avatar_upload_download_replace_and_delete_are_private_and_audited() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = avatar_path(&fixture);
    assert_error(
        target.get(&path, Some(&fixture.avatar_access_token)),
        404,
        "not_found",
    );
    let png = b"contract-png\0bytes".to_vec();
    let response = target.put_multipart(
        &path,
        &fixture.avatar_access_token,
        avatar_form(png.clone(), "first.png", "image/png"),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_no_store(&response);
    let uploaded_id = request_id(&response);
    assert_eq!(data(response)["avatar_attached"], true);
    download(
        &target,
        &path,
        &fixture.avatar_access_token,
        "image/png",
        "first.png",
        &png,
    );
    assert_error(target.get(&path, None), 401, "unauthorized");
    let foreign = assert_error(
        target.get(&path, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    assert!(!foreign.to_string().contains(&fixture.profile_email));

    let jpeg = b"contract-jpeg\0bytes".to_vec();
    let response = target.put_multipart(
        &path,
        &fixture.avatar_access_token,
        avatar_form(jpeg.clone(), "second.jpg", "image/jpeg"),
    );
    assert_eq!(response.status().as_u16(), 200);
    let replaced_id = request_id(&response);
    assert_eq!(data(response)["avatar_attached"], true);
    download(
        &target,
        &path,
        &fixture.avatar_access_token,
        "image/jpeg",
        "second.jpg",
        &jpeg,
    );

    let webp = b"contract-webp\0bytes".to_vec();
    let response = target.put_multipart(
        &path,
        &fixture.avatar_access_token,
        avatar_form(webp.clone(), "third.webp", "image/webp"),
    );
    assert_eq!(response.status().as_u16(), 200);
    let webp_id = request_id(&response);
    assert_eq!(data(response)["avatar_attached"], true);
    download(
        &target,
        &path,
        &fixture.avatar_access_token,
        "image/webp",
        "third.webp",
        &webp,
    );

    let response = target.delete(&path, Some(&fixture.avatar_access_token));
    assert_eq!(response.status().as_u16(), 204);
    assert_no_store(&response);
    let deleted_id = request_id(&response);
    assert!(response.bytes().expect("empty response").is_empty());
    assert_error(
        target.get(&path, Some(&fixture.avatar_access_token)),
        404,
        "not_found",
    );
    assert_eq!(
        data(target.get(
            &format!("/api/v1/households/{}/profile", fixture.avatar_household_id),
            Some(&fixture.avatar_access_token)
        ))["avatar_attached"],
        false
    );
    let audits = audit_rows(&target, &fixture);
    for id in [uploaded_id, replaced_id, webp_id] {
        assert!(audits
            .iter()
            .any(|row| row["request_id"] == id && row["event_type"] == "profile.avatar.updated"));
    }
    assert!(audits.iter().any(
        |row| row["request_id"] == deleted_id && row["event_type"] == "profile.avatar.removed"
    ));
}

#[test]
fn invalid_avatar_replacement_keeps_existing_bytes() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/profile/avatar",
        fixture.avatar_invalid_household_id
    );
    let original = b"avatar-original".to_vec();
    let response = target.put_multipart(
        &path,
        &fixture.avatar_invalid_access_token,
        avatar_form(original.clone(), "original.png", "image/png"),
    );
    assert_eq!(response.status().as_u16(), 200);
    for (form, code) in [
        (
            avatar_form(b"plain text".to_vec(), "bad.txt", "text/plain"),
            "validation_failed",
        ),
        (
            avatar_form(vec![b'x'; 5 * 1024 * 1024 + 1], "huge.png", "image/png"),
            "validation_failed",
        ),
        (
            multipart::Form::new().text("avatar", fixture.profile_signed_blob_id.clone()),
            "unprocessable_content",
        ),
    ] {
        assert_error(
            target.put_multipart(&path, &fixture.avatar_invalid_access_token, form),
            422,
            code,
        );
        download(
            &target,
            &path,
            &fixture.avatar_invalid_access_token,
            "image/png",
            "original.png",
            &original,
        );
    }
    let boundary = vec![b'v'; 5 * 1024 * 1024];
    let response = target.put_multipart(
        &path,
        &fixture.avatar_invalid_access_token,
        avatar_form(boundary.clone(), "boundary.png", "image/png"),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(data(response)["avatar_attached"], true);
    download(
        &target,
        &path,
        &fixture.avatar_invalid_access_token,
        "image/png",
        "boundary.png",
        &boundary,
    );
    assert_eq!(
        target
            .delete(&path, Some(&fixture.avatar_invalid_access_token))
            .status()
            .as_u16(),
        204
    );
}
