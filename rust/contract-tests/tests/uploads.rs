use medtracker_contract_tests::{fixture, Target};
use reqwest::blocking::Response;
use serde_json::json;
use url::Url;

const PREFIX: &str = "/rails/active_storage";
const ICON: &[u8] = include_bytes!("../../../public/icon.png");

fn disk_path(response: Response) -> String {
    assert_eq!(response.status().as_u16(), 302);
    let location = response.headers()["location"]
        .to_str()
        .expect("redirect URL");
    let url = Url::parse(location).expect("absolute disk URL");
    assert_eq!(
        url.path().split('/').take(4).collect::<Vec<_>>(),
        ["", "rails", "active_storage", "disk"]
    );
    assert!(url.query().is_none());
    assert!(url.fragment().is_none());
    url.path().to_owned()
}

fn assert_icon(response: Response) {
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.headers()["content-type"], "image/png");
    assert_eq!(response.bytes().expect("blob bytes").as_ref(), ICON);
}

#[test]
fn direct_upload_creation_and_disk_write_are_unavailable() {
    let target = Target::from_env();
    let response = target.post_json(
        &format!("{PREFIX}/direct_uploads"),
        &json!({"blob": {"filename": "contract-icon.png", "byte_size": ICON.len(),
            "content_type": "image/png", "checksum": "invalid"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.put_json_without_auth(
        &format!("{PREFIX}/disk/unsigned-token/contract-icon.png"),
        &json!({"bytes": "untrusted"}),
    );
    assert_eq!(response.status().as_u16(), 404);
}

#[test]
fn signed_blob_redirect_proxy_and_alias_download_public_bytes() {
    let target = Target::from_env();
    let fixture = fixture();
    let signed_id = &fixture.upload_blob_signed_id;
    for path in [
        format!("{PREFIX}/blobs/redirect/{signed_id}/contract-icon.png"),
        format!("{PREFIX}/blobs/{signed_id}/contract-icon.png"),
    ] {
        let disk = disk_path(target.get(&path, None));
        assert_icon(target.get(&disk, None));
        let encoded_key = disk
            .trim_start_matches(&format!("{PREFIX}/disk/"))
            .split('/')
            .next()
            .expect("disk key");
        let tampered_disk = disk.replacen(encoded_key, &format!("{encoded_key}x"), 1);
        assert_eq!(target.get(&tampered_disk, None).status().as_u16(), 404);
    }
    assert_icon(target.get(
        &format!("{PREFIX}/blobs/proxy/{signed_id}/contract-icon.png"),
        None,
    ));
}

#[test]
#[ignore = "Rails Active Storage has no configured variant transformer"]
fn signed_representation_redirect_proxy_and_alias_download_images() {
    let target = Target::from_env();
    let fixture = fixture();
    let signed_id = &fixture.upload_blob_signed_id;
    let variation = &fixture.upload_variation_key;
    for path in [
        format!("{PREFIX}/representations/redirect/{signed_id}/{variation}/contract-icon.png"),
        format!("{PREFIX}/representations/{signed_id}/{variation}/contract-icon.png"),
    ] {
        let disk = disk_path(target.get(&path, None));
        let response = target.get(&disk, None);
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(response.headers()["content-type"], "image/png");
        assert!(response
            .bytes()
            .expect("representation bytes")
            .starts_with(b"\x89PNG\r\n\x1a\n"));
    }
    let response = target.get(
        &format!("{PREFIX}/representations/proxy/{signed_id}/{variation}/contract-icon.png"),
        None,
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.headers()["content-type"], "image/png");
    assert!(response
        .bytes()
        .expect("representation bytes")
        .starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn tampered_blob_and_representation_signatures_do_not_download() {
    let target = Target::from_env();
    let fixture = fixture();
    let signed_id = format!("{}x", fixture.upload_blob_signed_id);
    for path in [
        format!("{PREFIX}/blobs/redirect/{signed_id}/contract-icon.png"),
        format!("{PREFIX}/blobs/proxy/{signed_id}/contract-icon.png"),
        format!("{PREFIX}/blobs/{signed_id}/contract-icon.png"),
        format!(
            "{PREFIX}/representations/redirect/{signed_id}/{}/contract-icon.png",
            fixture.upload_variation_key
        ),
        format!(
            "{PREFIX}/representations/proxy/{signed_id}/{}/contract-icon.png",
            fixture.upload_variation_key
        ),
        format!(
            "{PREFIX}/representations/{signed_id}/{}/contract-icon.png",
            fixture.upload_variation_key
        ),
    ] {
        let response = target.get(&path, None);
        assert_eq!(response.status().as_u16(), 404, "{path}");
    }
    assert_eq!(
        target
            .get(
                &format!("{PREFIX}/disk/unsigned-token/contract-icon.png"),
                None
            )
            .status()
            .as_u16(),
        404
    );
}
