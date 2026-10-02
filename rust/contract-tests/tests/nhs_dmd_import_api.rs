use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::{multipart, Response};
use serde_json::Value;
use std::io::Write;
use std::time::{Duration, Instant};

const AMPP_XML: &str = r#"<?xml version="1.0" encoding="utf-8" ?><ACTUAL_MEDICINAL_PROD_PACKS><AMPPS><AMPP><APPID>111</APPID><NM>Test Drug 10mg tablets (Acme Ltd) 28 tablet</NM><VPPID>501</VPPID><APID>9001</APID></AMPP><AMPP><APPID>222</APPID><NM>Second Drug 5mg tablets</NM><APID>9002</APID></AMPP><AMPP><APPID>333</APPID></AMPP></AMPPS></ACTUAL_MEDICINAL_PROD_PACKS>"#;

const GTIN_XML: &str = r#"<?xml version="1.0" encoding="utf-8" ?><GTIN_DETAILS><AMPPS><AMPP><AMPPID>111</AMPPID><GTINDATA><GTIN>05012345678901</GTIN><STARTDT>2020-01-01</STARTDT></GTINDATA><GTINDATA><GTIN>05012345678902</GTIN><STARTDT>2020-01-01</STARTDT><ENDDT>2020-01-02</ENDDT></GTINDATA></AMPP><AMPP><AMPPID>999</AMPPID><GTINDATA><GTIN>05012345678903</GTIN></GTINDATA></AMPP><AMPP><GTINDATA><GTIN>05012345678904</GTIN></GTINDATA></AMPP></AMPPS></GTIN_DETAILS>"#;

const GTIN_XML_DUPLICATE: &str = r#"<?xml version="1.0" encoding="utf-8" ?><GTIN_DETAILS><AMPPS><AMPP><AMPPID>111</AMPPID><GTINDATA><GTIN>05012345678911</GTIN><STARTDT>2020-01-01</STARTDT></GTINDATA><GTINDATA><GTIN>0501 2345 678911</GTIN><STARTDT>2020-01-01</STARTDT></GTINDATA></AMPP></AMPPS></GTIN_DETAILS>"#;

fn nested_gtin_zip(xml: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        writer
            .start_file(
                "f_gtin2_0240926.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .expect("gtin file entry");
        writer.write_all(xml.as_bytes()).expect("gtin xml bytes");
        writer.finish().expect("finish gtin zip");
    }
    buffer
}

fn release_zip_with(gtin_xml: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        writer
            .start_file(
                "f_ampp2_3240926.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .expect("ampp entry");
        writer.write_all(AMPP_XML.as_bytes()).expect("ampp xml");
        writer
            .start_file(
                "week402026-r2_3-GTIN.zip",
                zip::write::SimpleFileOptions::default(),
            )
            .expect("nested zip entry");
        writer
            .write_all(&nested_gtin_zip(gtin_xml))
            .expect("nested gtin bytes");
        writer.finish().expect("finish release zip");
    }
    buffer
}

fn release_zip() -> Vec<u8> {
    release_zip_with(GTIN_XML)
}

fn import_form(bytes: Vec<u8>, filename: &str) -> multipart::Form {
    let part = multipart::Part::bytes(bytes)
        .file_name(filename.to_owned())
        .mime_str("application/zip")
        .expect("zip MIME");
    multipart::Form::new().part("release_zip", part)
}

fn base(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/admin/nhs_dmd_imports",
        fixture.platform_household_id
    )
}

fn data(response: Response) -> Value {
    response.json::<Value>().expect("JSON body")["data"].clone()
}

fn show_run(target: &Target, fixture: &Fixture, id: i64) -> Value {
    let response = target.get(
        &format!("{}/{id}", base(fixture)),
        Some(&fixture.platform_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    data(response)
}

fn wait_for_completion(target: &Target, fixture: &Fixture, id: i64) -> Value {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let run = show_run(target, fixture, id);
        if run["active"] == false {
            return run;
        }
        assert!(Instant::now() < deadline, "import did not finish in time");
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn upload(target: &Target, fixture: &Fixture, bytes: Vec<u8>, filename: &str) -> Response {
    target.post_multipart(
        &base(fixture),
        &fixture.platform_access_token,
        import_form(bytes, filename),
    )
}

#[test]
fn nhs_dmd_import_upload_runs_and_reports_counts() {
    let target = Target::from_env();
    let fixture = fixture();

    let denied = target.get(&base(&fixture), Some(&fixture.access_token));
    assert_eq!(denied.status().as_u16(), 403);
    let denied = target.get(
        &format!(
            "/api/v1/households/{}/admin/nhs_dmd_imports",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(denied.status().as_u16(), 403);

    let response = upload(&target, &fixture, release_zip(), "release-test.zip");
    assert_eq!(response.status().as_u16(), 201);
    let created = data(response);
    let run_id = created["id"].as_i64().expect("run id");

    let run = wait_for_completion(&target, &fixture, run_id);
    assert_eq!(run["status"], "completed");
    assert_eq!(run["uploaded_filename"], "release-test.zip");
    assert_eq!(run["created_count"], 1);
    assert_eq!(run["skipped_expired_count"], 1);
    assert_eq!(run["skipped_missing_name_count"], 2);
    assert_eq!(run["imported_count"], 1);
    assert_eq!(run["skipped_count"], 3);
    assert!(run["total_records"].as_i64().unwrap() >= 6);

    let second = upload(&target, &fixture, release_zip(), "release-again.zip");
    assert_eq!(second.status().as_u16(), 201);
    let second_id = data(second)["id"].as_i64().expect("second run id");
    let rerun = wait_for_completion(&target, &fixture, second_id);
    assert_eq!(rerun["status"], "completed");
    assert_eq!(rerun["unchanged_count"], 1);
    assert_eq!(rerun["created_count"], 0);

    let list = target.get(&base(&fixture), Some(&fixture.platform_access_token));
    assert_eq!(list.status().as_u16(), 200);
    let rows = data(list).as_array().expect("run list").clone();
    assert!(rows.len() >= 2);
    assert_eq!(rows[0]["id"].as_i64(), Some(second_id));
}

#[test]
fn nhs_dmd_import_duplicate_gtin_is_skipped() {
    let target = Target::from_env();
    let fixture = fixture();

    let response = upload(
        &target,
        &fixture,
        release_zip_with(GTIN_XML_DUPLICATE),
        "release-duplicate.zip",
    );
    assert_eq!(response.status().as_u16(), 201);
    let run_id = data(response)["id"].as_i64().expect("run id");
    let run = wait_for_completion(&target, &fixture, run_id);
    assert_eq!(run["status"], "completed");
    assert_eq!(run["created_count"], 1);
    assert_eq!(run["skipped_invalid_count"], 1);
}

#[test]
fn nhs_dmd_import_invalid_archive_fails() {
    let target = Target::from_env();
    let fixture = fixture();

    let response = upload(&target, &fixture, b"not a zip".to_vec(), "broken.zip");
    assert_eq!(response.status().as_u16(), 201);
    let run_id = data(response)["id"].as_i64().expect("run id");
    let run = wait_for_completion(&target, &fixture, run_id);
    assert_eq!(run["status"], "failed");
    assert!(run["error_message"].as_str().unwrap().contains("ZIP"));
}

#[test]
fn nhs_dmd_import_missing_file_rejected() {
    let target = Target::from_env();
    let fixture = fixture();
    let denied = target.post_multipart(
        &base(&fixture),
        &fixture.access_token,
        multipart::Form::new().text("note", "no file here"),
    );
    assert_eq!(denied.status().as_u16(), 403);
    let form = multipart::Form::new().text("note", "no file here");
    let response = target.post_multipart(&base(&fixture), &fixture.platform_access_token, form);
    assert_eq!(response.status().as_u16(), 422);
}
