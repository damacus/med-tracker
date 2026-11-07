use crate::harness::{self, named};
use playwright_rs::expect;
use playwright_rs::protocol::{
    Animations, AriaRole, FilePayload, Page, ScreenshotOptions, Viewport,
};
use sea_orm::ConnectionTrait;

async fn prepare_foreign_login(journey: &harness::Journey) {
    use med_tracker::models::_entities::people;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set};

    people::ActiveModel {
        id: Set(73005),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(1982, 1, 1)),
        ..Default::default()
    }
    .update(&journey.admin)
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn import_confirmation_uses_real_preview_and_replays_without_duplicate_people() {
    use med_tracker::models::{_entities::household_memberships, entities::person};
    use sea_orm::{ActiveValue::Set, EntityTrait, PaginatorTrait};
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/admin/data_imports/new", journey.origin), None).await.unwrap();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in [("portable.json", include_bytes!("../fixtures/portable_rails/prepared-history-v2.json").as_slice()), ("manifest.json", include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json").as_slice())] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        let bytes = zip.finish().unwrap().into_inner();
        page.locator("#archive").set_input_files_payload(FilePayload::new("export.zip", "application/zip", bytes.clone()), None).await.unwrap();
        page.locator("#passphrase").fill("password", None).await.unwrap();
        page.locator("[name=timezone_confirmed]").click(None).await.unwrap();
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("main h1")).to_contain_text("Map people").await.unwrap();
        page.locator("select[name^='mapping[']").select_option("new_dependent", None).await.unwrap();
        page.locator("#archive").set_input_files_payload(FilePayload::new("export.zip", "application/zip", bytes), None).await.unwrap();
        page.locator("#passphrase").fill("password", None).await.unwrap();
        page.locator("[name=timezone_confirmed]").click(None).await.unwrap();
        page.evaluate::<(), ()>("() => document.querySelectorAll('[name=acknowledgement]').forEach(c => c.checked=true)", None).await.unwrap();
        let blocked: bool = page.evaluate("async () => {const f=document.querySelector('#administration-form'); const r=await fetch(f.action,{method:'POST',body:new FormData(f)}); const h=await r.text(); return h.includes('Blocked') && !h.includes('name=\"preview_token\"');}", None::<&()>).await.unwrap();
        assert!(blocked);
        page.locator("input[name^='guardian[']").click(None).await.unwrap();
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Ready").await.unwrap();
        let wrong: bool = page.evaluate("async () => {const f=document.querySelector('#administration-form'); const d=new URLSearchParams(new FormData(f)); d.set('preview_token','AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'); d.set('passphrase','password'); d.set('confirm','true'); const r=await fetch(f.action,{method:'POST',body:d}); const h=await r.text(); return r.status===403 && !h.includes('password') && !h.includes('AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA');}", None::<&()>).await.unwrap();
        assert!(wrong);
        let replay: serde_json::Value = page.evaluate("() => {const f=document.querySelector('#administration-form'); return {url:f.action,csrf:f.elements.authenticity_token.value,token:f.elements.preview_token.value};}", None::<&()>).await.unwrap();
        let ready_history: bool = page.evaluate("async x => {const r=await fetch(x.url.replace(/\\/confirm$/, '')); const h=await r.text(); return r.ok && h.includes('Ready') && h.includes('name=\"preview_token\"');}", Some(&replay)).await.unwrap();
        assert!(ready_history);
        assert!(page.evaluate::<(), bool>("() => !!document.querySelector('label[for=passphrase]') && !!document.querySelector('label:has([name=confirm])')", None).await.unwrap());
        page.locator("#passphrase").focus().await.unwrap();
        page.keyboard().press("Tab", None).await.unwrap();
        assert!(page.evaluate::<(), bool>("() => document.activeElement.name === 'confirm'", None).await.unwrap());
        let screenshots = std::path::Path::new("docs/screenshots/clinical-data-import");
        std::fs::create_dir_all(screenshots).unwrap();
        for (name, width, height) in [("ready-desktop.png", 1440, 1000), ("ready-mobile.png", 390, 844)] {
            page.set_viewport_size(Viewport { width, height }).await.unwrap();
            if width == 390 {
                assert!(page.evaluate::<(), bool>("() => document.documentElement.scrollWidth <= window.innerWidth", None).await.unwrap(), "ready report must fit the mobile viewport");
            }
            page.screenshot_to_file(&screenshots.join(name), ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
        }
        page.set_viewport_size(Viewport { width: 1440, height: 1000 }).await.unwrap();
        page.locator("#passphrase").fill("password", None).await.unwrap();
        page.locator("[name=confirm]").click(None).await.unwrap();
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Completed").await.unwrap();
        assert_eq!(page.locator("input[name=preview_token]").count().await.unwrap(), 0);
        for (name, width, height) in [("completed-desktop.png", 1440, 1000), ("completed-mobile.png", 390, 844)] {
            page.set_viewport_size(Viewport { width, height }).await.unwrap();
            if width == 390 {
                assert!(page.evaluate::<(), bool>("() => document.documentElement.scrollWidth <= window.innerWidth", None).await.unwrap(), "completed report must fit the mobile viewport");
            }
            page.screenshot_to_file(&screenshots.join(name), ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
        }
        let count = person::Entity::find().count(&journey.admin).await.unwrap();
        let replayed: bool = page.evaluate("async x => {const d=new URLSearchParams({authenticity_token:x.csrf,preview_token:x.token,passphrase:'password',confirm:'true'}); const r=await fetch(x.url,{method:'POST',body:d}); return r.ok && (await r.text()).includes('Completed');}", Some(&replay)).await.unwrap();
        assert!(replayed);
        assert_eq!(person::Entity::find().count(&journey.admin).await.unwrap(), count);
        let completed_history: bool = page.evaluate("async x => {const r=await fetch(x.url.replace(/\\/confirm$/, '')); const h=await r.text(); return r.ok && h.includes('Completed') && !h.includes('name=\"preview_token\"');}", Some(&replay)).await.unwrap();
        assert!(completed_history);
        let now = chrono::Utc::now().naive_utc();
        household_memberships::Entity::insert(household_memberships::ActiveModel {
            id: Set(74801), household_id: Set(72001), person_id: Set(73005), role: Set("member".into()),
            status: Set("active".into()), joined_at: Set(now), created_at: Set(now), updated_at: Set(now), ..Default::default()
        }).exec(&journey.admin).await.unwrap();
        prepare_foreign_login(journey).await;
        page.context().unwrap().clear_cookies(None).await.unwrap();
        page.goto(&format!("{}/login", journey.origin), None).await.unwrap();
        page.get_by_label("Email address", true).fill("foreign@example.test", None).await.unwrap();
        page.get_by_label("Password", true).fill("password", None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Sign in")).click(None).await.unwrap();
        assert!(page.evaluate::<(), bool>("async () => { const r=await fetch('/account/security'); return r.ok && !r.redirected; }", None).await.unwrap(), "foreign sign-in must succeed");
        page.goto(&format!("{}/account/security", journey.origin), None).await.unwrap();
        let member_csrf: String = page.evaluate("() => document.querySelector('[name=authenticity_token]').value", None::<&()>).await.unwrap();
        let denied_input = serde_json::json!({"url":replay["url"],"token":replay["token"],"csrf":member_csrf});
        let denied: u16 = page.evaluate("async x => {const d=new URLSearchParams({authenticity_token:x.csrf,preview_token:x.token,passphrase:'password',confirm:'true'}); return (await fetch(x.url,{method:'POST',body:d})).status;}", Some(&denied_input)).await.unwrap();
        assert!(matches!(denied, 403 | 404));
        assert_eq!(person::Entity::find().count(&journey.admin).await.unwrap(), count);
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn import_consent_recipient_approval_withdrawal_and_foreign_denial() {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use med_tracker::models::_entities::household_memberships;
    use sea_orm::{ActiveValue::Set, EntityTrait};
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        let now = chrono::Utc::now().naive_utc();
        household_memberships::Entity::insert(household_memberships::ActiveModel {
            id: Set(74801), household_id: Set(72001), person_id: Set(73005), role: Set("member".into()),
            status: Set("active".into()), joined_at: Set(now), created_at: Set(now), updated_at: Set(now), ..Default::default()
        }).exec(&journey.admin).await.unwrap();
        prepare_foreign_login(journey).await;
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/admin/data_imports/new", journey.origin), None).await.unwrap();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in [("portable.json", include_bytes!("../fixtures/portable_rails/prepared-history-v2.json").as_slice()), ("manifest.json", include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json").as_slice())] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        page.locator("#archive").set_input_files_payload(FilePayload::new("export.zip", "application/zip", zip.finish().unwrap().into_inner()), None).await.unwrap();
        page.locator("#passphrase").fill("password", None).await.unwrap();
        page.locator("[name=timezone_confirmed]").click(None).await.unwrap();
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("main h1")).to_contain_text("Map people").await.unwrap();
        let request_form = page.locator("form[action$='/admin/data_import_consents']").nth(0);
        expect(request_form.locator("select")).to_contain_text("Synthetic foreign adult").await.unwrap();
        assert!(page.evaluate::<(), bool>("() => [...document.querySelectorAll('#administration-form select[name^=mapping] option')].some(o => o.value==='73005' && o.textContent.includes('Synthetic foreign adult'))", None).await.unwrap());
        request_form.locator("select").select_option("73005", None).await.unwrap();
        let binding: serde_json::Value = page.evaluate("() => {const f=document.querySelector('form[action$=\"/admin/data_import_consents\"]'); const k=f.elements.source_key.value.replace(/-/g,'+').replace(/_/g,'/'); return {source_sha256:f.elements.source_sha256.value,source_person:JSON.parse(atob(k)),target_person:73005};}", None::<&()>).await.unwrap();
        let expected = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&binding).unwrap());
        let csrf: String = page.evaluate("() => document.querySelector('form[action$=\"/admin/data_import_consents\"] [name=authenticity_token]').value", None::<&()>).await.unwrap();
        let cookies = page.context().unwrap().cookies(None).await.unwrap().into_iter().map(|cookie| format!("{}={}", cookie.name, cookie.value)).collect::<Vec<_>>().join("; ");
        request_form.locator("button[type=submit]").click(None).await.unwrap();
        expect(page.locator("main h1")).to_contain_text("Consent request created").await.unwrap();
        let path = page.locator("main a[href*='/data_import_consents/']").get_attribute("href").await.unwrap().unwrap();
        let denied_get = reqwest::Client::new().get(format!("{}{path}", journey.origin)).header("cookie", &cookies).send().await.unwrap();
        assert!(matches!(denied_get.status().as_u16(), 403 | 404));
        let mut form = url::form_urlencoded::Serializer::new(String::new());
        form.append_pair("authenticity_token", &csrf).append_pair("expected", &expected).append_pair("decision", "accept");
        let denied = reqwest::Client::new().post(format!("{}{path}", journey.origin)).header("cookie", &cookies).header("content-type", "application/x-www-form-urlencoded").body(form.finish()).send().await.unwrap();
        assert!(matches!(denied.status().as_u16(), 403 | 404));
        page.context().unwrap().clear_cookies(None).await.unwrap();
        page.goto(&format!("{}/login", journey.origin), None).await.unwrap();
        page.get_by_label("Email address", true).fill("foreign@example.test", None).await.unwrap();
        page.get_by_label("Password", true).fill("password", None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Sign in")).click(None).await.unwrap();
        assert!(page.evaluate::<(), bool>("async () => { const r=await fetch('/account/security'); return r.ok && !r.redirected; }", None).await.unwrap(), "foreign sign-in must succeed");
        let locale_preserved: bool = page.evaluate("async path => { const url=new URL(path,location.origin); url.search=''; const r=await fetch(url,{headers:{'Accept-Language':'es'}}); const doc=new DOMParser().parseFromString(await r.text(),'text/html'); const f=doc.querySelector('#administration-form'); if(!f || f.elements.lang.value!=='es') return false; const d=new URLSearchParams(new FormData(f)); d.set('decision','invalid'); const response=await fetch(f.action,{method:'POST',body:d}); return response.status===403 && (await response.text()).includes('lang=\"es\"'); }", Some(&path)).await.unwrap();
        assert!(locale_preserved);
        page.goto(&format!("{}{path}", journey.origin), None).await.unwrap();
        expect(page.locator("main h1")).to_contain_text("Import consent request").await.unwrap();
        page.locator("input[name=decision][value=accept]").click(None).await.unwrap();
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Accepted").await.unwrap();
        let foreign_path = path.replace("persistence-fixture", "foreign-fixture");
        let denied: u16 = page.evaluate("async p => (await fetch(p)).status", Some(&foreign_path)).await.unwrap();
        assert!(matches!(denied, 403 | 404));
        let recipient_csrf: String = page.evaluate("() => document.querySelector('#administration-form [name=authenticity_token]').value", None::<&()>).await.unwrap();
        let recipient_cookies = page.context().unwrap().cookies(None).await.unwrap().into_iter().map(|cookie| format!("{}={}", cookie.name, cookie.value)).collect::<Vec<_>>().join("; ");
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Rejected").await.unwrap();
        assert_eq!(page.locator("#administration-form").count().await.unwrap(), 0);
        let mut replay = url::form_urlencoded::Serializer::new(String::new());
        replay.append_pair("authenticity_token", &recipient_csrf).append_pair("expected", &expected).append_pair("decision", "accept");
        let denied_replay = reqwest::Client::new().post(format!("{}{path}", journey.origin)).header("cookie", recipient_cookies).header("content-type", "application/x-www-form-urlencoded").body(replay.finish()).send().await.unwrap();
        assert_eq!(denied_replay.status().as_u16(), 409);
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn adult_preview_explains_absent_and_pending_consent() {
    use med_tracker::models::_entities::{household_memberships, people};
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        let now = chrono::Utc::now().naive_utc();
        household_memberships::Entity::insert(household_memberships::ActiveModel {
            id: Set(74801), household_id: Set(72001), person_id: Set(73005), role: Set("member".into()),
            status: Set("active".into()), joined_at: Set(now), created_at: Set(now), updated_at: Set(now), ..Default::default()
        }).exec(&journey.admin).await.unwrap();
        people::ActiveModel { id: Set(73005), date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(1982, 1, 1)), ..Default::default() }
            .update(&journey.admin).await.unwrap();
        let mut payload: serde_json::Value = serde_json::from_slice(include_bytes!("../fixtures/portable_rails/prepared-history-v2.json")).unwrap();
        let records = payload["records"].as_object_mut().unwrap();
        for (key, value) in records.iter_mut() { if key != "people" { *value = serde_json::json!([]); } }
        let person = &mut records.get_mut("people").unwrap()[0];
        person["person_type"] = serde_json::json!("adult");
        person["has_capacity"] = serde_json::json!(true);
        person["date_of_birth"] = serde_json::json!("1982-01-01");
        person["notification_preference_portable_id"] = serde_json::Value::Null;
        let mut manifest: serde_json::Value = serde_json::from_slice(include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json")).unwrap();
        for (key, count) in manifest["record_counts"].as_object_mut().unwrap() { *count = serde_json::json!(if key == "people" { 1 } else { 0 }); }
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in [("portable.json", serde_json::to_vec(&payload).unwrap()), ("manifest.json", serde_json::to_vec(&manifest).unwrap())] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(&bytes).unwrap();
        }
        let archive = zip.finish().unwrap().into_inner();
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/admin/data_imports/new", journey.origin), None).await.unwrap();
        page.locator("#archive").set_input_files_payload(FilePayload::new("export.zip", "application/zip", archive.clone()), None).await.unwrap();
        page.locator("#passphrase").fill("password", None).await.unwrap();
        page.locator("[name=timezone_confirmed]").click(None).await.unwrap();
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("main h1")).to_contain_text("Map people").await.unwrap();
        page.locator("select[name^='mapping[']").select_option("73005", None).await.unwrap();
        page.locator("#archive").set_input_files_payload(FilePayload::new("export.zip", "application/zip", archive), None).await.unwrap();
        page.locator("#passphrase").fill("password", None).await.unwrap();
        page.locator("[name=timezone_confirmed]").click(None).await.unwrap();
        let absent: bool = page.evaluate("async () => {const f=document.querySelector('#administration-form'); const r=await fetch(f.action,{method:'POST',body:new FormData(f)}); const h=await r.text(); return r.status===200 && h.includes('Blocked') && h.includes('Current consent is required.') && !h.includes('name=\"preview_token\"') && !h.includes('Synthetic foreign adult');}", None::<&()>).await.unwrap();
        assert!(absent);
        page.locator("form[action$='/admin/data_import_consents'] select").select_option("73005", None).await.unwrap();
        let reference: String = page.evaluate("async () => {const f=document.querySelector('form[action$=\"/admin/data_import_consents\"]'); const r=await fetch(f.action,{method:'POST',body:new URLSearchParams(new FormData(f))}); if(!r.ok) return ''; const doc=new DOMParser().parseFromString(await r.text(),'text/html'); return doc.querySelector('main code')?.textContent.trim() ?? '';}", None::<&()>).await.unwrap();
        assert!(!reference.is_empty());
        page.locator("input[name^='recipient_ref[']").fill(&reference, None).await.unwrap();
        let pending: bool = page.evaluate("async () => {const f=document.querySelector('#administration-form'); const r=await fetch(f.action,{method:'POST',body:new FormData(f)}); const h=await r.text(); return r.status===200 && h.includes('Blocked') && h.includes('Current consent is required.') && !h.includes('name=\"preview_token\"') && !h.includes('Synthetic foreign adult');}", None::<&()>).await.unwrap();
        assert!(pending);
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn revoked_browser_session_during_upload_cannot_persist_inspection() {
    use med_tracker::models::{_entities::data_import_inspections, entities::identity_session};
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, sea_query::Expr};
    use std::io::{Cursor, Write};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        time::{Duration, timeout},
    };
    use zip::{ZipWriter, write::SimpleFileOptions};

    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/admin/data_imports/new", journey.origin), None).await.unwrap();
        let csrf: String = page.evaluate("() => document.querySelector('[name=authenticity_token]').value", None::<&()>).await.unwrap();
        let cookies = page.context().unwrap().cookies(None).await.unwrap().into_iter().map(|cookie| format!("{}={}", cookie.name, cookie.value)).collect::<Vec<_>>().join("; ");
        let before = data_import_inspections::Entity::find().filter(data_import_inspections::Column::HouseholdId.eq(72001)).count(&journey.admin).await.unwrap();
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in [("portable.json", include_bytes!("../fixtures/portable_rails/prepared-history-v2.json").as_slice()), ("manifest.json", include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json").as_slice())] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        let archive = zip.finish().unwrap().into_inner();
        let boundary = "test-import-boundary";
        let mut body = Vec::new();
        for (name, value) in [("authenticity_token", csrf.as_str()), ("source", "med_tracker05"), ("passphrase", "password"), ("source_timezone", "Europe/London"), ("timezone_confirmed", "true")] {
            body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
        }
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"archive\"; filename=\"fixture.zip\"\r\nContent-Type: application/zip\r\n\r\n").as_bytes());
        body.extend_from_slice(&archive);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let address = journey.origin.strip_prefix("http://").unwrap();
        let path = "/households/persistence-fixture/admin/data_imports/new";
        let mut socket = TcpStream::connect(address).await.unwrap();
        socket.write_all(format!("POST {path} HTTP/1.1\r\nHost: {address}\r\nCookie: {cookies}\r\nContent-Type: multipart/form-data; boundary={boundary}\r\nContent-Length: {}\r\n\r\n", body.len()).as_bytes()).await.unwrap();
        socket.write_all(&body[..1]).await.unwrap();
        let client = reqwest::Client::new();
        let mut acquired = false;
        for _ in 0..100 {
            let response = client.post(format!("{}{path}", journey.origin)).header("cookie", &cookies).header("content-type", "multipart/form-data; boundary=probe").body("--probe--\r\n").send().await.unwrap();
            if response.status().as_u16() == 503 { acquired = true; break; }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(acquired, "the upload must be paused after acquiring the permit");
        identity_session::Entity::update_many().col_expr(identity_session::Column::Active, Expr::value(false)).filter(identity_session::Column::UserId.eq(71001)).filter(identity_session::Column::Active.eq(true)).exec(&journey.admin).await.unwrap();
        socket.write_all(&body[1..]).await.unwrap();
        let mut reply = [0_u8; 512];
        let size = timeout(Duration::from_secs(10), socket.read(&mut reply)).await.unwrap().unwrap();
        let status = std::str::from_utf8(&reply[..size]).unwrap();
        assert!(status.starts_with("HTTP/1.1 401") || status.starts_with("HTTP/1.1 403"));
        let after = data_import_inspections::Entity::find().filter(data_import_inspections::Column::HouseholdId.eq(72001)).count(&journey.admin).await.unwrap();
        assert_eq!(after, before);
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn import_preview_post_requires_auth_and_rejects_invalid_mapping_input() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        let page = &journey.page;
        page.goto(&format!("{}/login", journey.origin), None).await.unwrap();
        assert!(page.evaluate::<(), bool>("async () => { const r = await fetch('/households/persistence-fixture/admin/data_imports/preview', {method:'POST', body:new FormData()}); return r.url.endsWith('/login'); }", None).await.unwrap());
        harness::sign_in(journey).await;
        page.goto(&format!("{}/households/persistence-fixture/admin/data_imports/new", journey.origin), None).await.unwrap();
        assert!(page.evaluate::<(), bool>("async () => { const f = new FormData(); f.append('authenticity_token', document.querySelector('[name=authenticity_token]').value); f.append('inspection_token', 'cfae5719-6509-4a7e-8e12-64b4e1684002'); f.append('source','other'); f.append('source_timezone','Europe/Paris'); f.append('timezone_confirmed','true'); f.append('passphrase','preview-secret'); f.append('archive',new Blob(['invalid']),'input.zip'); const r=await fetch('/households/persistence-fixture/admin/data_imports/preview',{method:'POST',body:f}); const body=await r.text(); return r.status===422 && body.includes('Europe/Paris') && !body.includes('preview-secret'); }", None).await.unwrap());
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn import_upload_authentication_and_failed_archive_do_not_expose_passphrase() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        let page = &journey.page;
        let url = format!("{}/households/persistence-fixture/admin/data_imports/new", journey.origin);
        page.goto(&url, None).await.unwrap();
        assert!(page.url().ends_with("/login"));
        harness::sign_in(journey).await;
        page.goto(&url, None).await.unwrap();
        page.locator("#archive").set_input_files_payload(FilePayload::new("invalid.zip", "application/zip", vec![1, 2, 3]), None).await.unwrap();
        page.locator("#passphrase").fill("synthetic-secret", None).await.unwrap();
        page.locator("#source_timezone").fill("America/New_York", None).await.unwrap();
        page.locator("[name=timezone_confirmed]").click(None).await.unwrap();
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("[role=alert]")).to_be_visible().await.unwrap();
        assert!(page.evaluate::<(), bool>("() => document.querySelector('#source_timezone').value === 'America/New_York' && document.querySelector('#passphrase').value === '' && document.querySelector('#archive').files.length === 0 && !document.querySelector('[name=timezone_confirmed]').checked && !document.body.textContent.includes('synthetic-secret')", None).await.unwrap());
        page.locator("#archive").set_input_files_payload(FilePayload::new("invalid.zip", "application/zip", vec![1, 2, 3]), None).await.unwrap();
        page.locator("#passphrase").fill("validation-secret", None).await.unwrap();
        page.locator("#source_timezone").fill("Europe/Paris", None).await.unwrap();
        page.locator("[name=timezone_confirmed]").click(None).await.unwrap();
        assert!(page.evaluate::<(), bool>("() => { const s=document.querySelector('#source'); s.add(new Option('Other','other',true,true)); return s.value==='other'; }", None).await.unwrap());
        page.locator("#administration-form button[type=submit]").click(None).await.unwrap();
        expect(page.locator("[role=alert]")).to_be_visible().await.unwrap();
        assert!(page.evaluate::<(), bool>("() => document.querySelector('#source_timezone').value === 'Europe/Paris' && document.querySelector('#passphrase').value === '' && document.querySelector('#archive').files.length === 0 && !document.querySelector('[name=timezone_confirmed]').checked && !document.body.textContent.includes('validation-secret')", None).await.unwrap());
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn import_reports_require_sign_in_and_open_for_household_admin() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                let url = format!(
                    "{}/households/persistence-fixture/admin/data_imports",
                    journey.origin
                );
                page.goto(&url, None).await.unwrap();
                assert!(page.url().ends_with("/login"));
                harness::sign_in(journey).await;
                page.goto(&url, None).await.unwrap();
                expect(page.locator("main h1"))
                    .to_contain_text("Import report")
                    .await
                    .unwrap();
            })
        },
    )
    .await;
}

async fn capture(page: &Page, name: &str) {
    if page.locator("body.app-household").count().await.unwrap() > 0 {
        expect(page.locator("body[data-navigation-ready]"))
            .to_be_visible()
            .await
            .unwrap();
    }
    let overflowing = page.evaluate::<(), serde_json::Value>("() => [...document.querySelectorAll('body *')].filter(e => e.getBoundingClientRect().right > innerWidth).map(e => ({tag:e.tagName,classes:e.className,text:e.textContent.slice(0,80),right:e.getBoundingClientRect().right}))",None).await.unwrap();
    assert!(
        !page
            .evaluate::<(), bool>(
                "() => document.documentElement.scrollWidth > innerWidth",
                None
            )
            .await
            .unwrap(),
        "overflow: {name}: {overflowing}"
    );
    assert!(page.evaluate::<(), bool>("() => { const ids = [...document.querySelectorAll('[id]')].map(e=>e.id); return ids.length === new Set(ids).size; }", None).await.unwrap(), "duplicate ids: {name}");
    let directory = std::path::Path::new("docs/screenshots/shared-layout");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("admin-{name}.png")),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .full_page(name.starts_with("overview") || name.contains("error"))
            .build(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn administration_polish_overview_and_forms() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| Box::pin(async move {
        let page=&journey.page;
        journey.admin.execute_unprepared("INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87801,72001,'synthetic-expired@example.test','member','polish-expired',74001,now()-interval '1 day',now(),now()); INSERT INTO security_audit_events(household_id,actor_account_id,actor_membership_id,event_type,metadata,audit_context,created_at,updated_at) VALUES(72001,71001,74001,'household_membership.role_updated','{\"secret\":\"private clinical sentinel\"}','{}',now(),now())").await.unwrap();
        harness::sign_in(journey).await;
        page.goto(&format!("{}/households/persistence-fixture/admin",journey.origin),None).await.unwrap();
        expect(page.locator("#expired-invitations")).to_contain_text("Expired invitations").await.unwrap();
        expect(page.locator(".admin-metrics")).to_contain_text("ACTIVE MEMBERS").await.unwrap();
        expect(page.locator(".admin-metrics")).to_contain_text("PENDING INVITATIONS").await.unwrap();
        expect(page.locator(".admin-metrics")).to_contain_text("PEOPLE NEEDING CARERS").await.unwrap();
        assert_eq!(page.locator(".admin-action-row").count().await.unwrap(),7);
        assert!(!page.locator("main").text_content().await.unwrap().unwrap().contains("private clinical sentinel"));
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        harness::device_appearance(page, "dark").await;
        assert!(page.locator("html").get_attribute("data-theme").await.unwrap().unwrap().ends_with("-dark"));
        capture(page,"overview-dark-desktop").await;
        harness::device_appearance(page, "light").await;
        page.locator("#expired-invitations a").click(None).await.unwrap();
        expect(page.locator("#invitation-form")).to_be_visible().await.unwrap();
        page.get_by_label("Email",true).fill("synthetic-invite@example.test",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Send invitation")).click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Invitation email queued for delivery.").await.unwrap();
        for (path,label) in [("","overview"),("/users","members"),("/invitations","invitations"),("/household/edit","settings"),("/users/74001/edit","role"),("/carer_relationships","care"),("/carer_relationships/new","care-form"),("/audit_logs","audit")] {
            page.goto(&format!("{}/households/persistence-fixture/admin{path}",journey.origin),None).await.unwrap();
            expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
            if label == "overview" { expect(page.locator(".admin-activity")).to_contain_text("Invitation created").await.unwrap(); }
            for width in [1440,390,320] {
                page.set_viewport_size(Viewport {width,height:if width==1440 {1000}else{844}}).await.unwrap();
                capture(page,&format!("{label}-{width}")).await;
            }
            if label == "invitations" {
                let trigger = page.locator("button[data-dialog-open='cancel-invitation-87801']");
                trigger.click(None).await.unwrap();
                expect(page.locator("dialog[open]")).to_be_visible().await.unwrap();
                capture(page,"invitation-dialog-320").await;
                page.keyboard().press("Escape",None).await.unwrap();
                expect(trigger).to_be_focused().await.unwrap();
            }
        }
        page.set_viewport_size(Viewport { width: 1440, height: 1000 }).await.unwrap();
        let detail = page.locator("main table tbody tr:first-child a");
        detail.click(None).await.unwrap();
        expect(page.locator("dialog[open]")).to_be_visible().await.unwrap();
        assert!(!page.evaluate::<(), bool>("() => [...document.querySelectorAll('dialog[open] dt')].some(term => term.textContent.trim() === 'Event type')", None).await.unwrap());
        assert!(!page.evaluate::<(), bool>("() => [...document.querySelectorAll('dialog[open] dt')].some(term => ['When (UTC)', 'Actor'].includes(term.textContent.trim()))", None).await.unwrap());
        assert!(page.url().ends_with("/admin/audit_logs"));
        for width in [1440,390,320] {
            page.set_viewport_size(Viewport {width,height:if width==1440 {1000}else{844}}).await.unwrap();
            capture(page,&format!("audit-detail-{width}")).await;
        }
        page.set_viewport_size(Viewport { width: 1440, height: 1000 }).await.unwrap();
        page.keyboard().press("Escape", None).await.unwrap();
        expect(detail).to_be_focused().await.unwrap();
        page.goto(&format!("{}/households/persistence-fixture/admin/household/edit",journey.origin),None).await.unwrap();
        page.get_by_label("Household name",true).fill("   ",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Save household")).click(None).await.unwrap();
        expect(page.locator("[role=alert]")).to_be_visible().await.unwrap();
        capture(page,"settings-error-320").await;
        page.get_by_label("Household name",true).fill("Synthetic renamed household",None).await.unwrap();
        page.get_by_role(AriaRole::Button,named("Save household")).click(None).await.unwrap();
        expect(page.get_by_label("Household name",true)).to_have_value("Synthetic renamed household").await.unwrap();
        page.goto(&format!("{}/households/persistence-fixture/admin/audit_logs?event_type=nonexistent",journey.origin),None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        expect(page.locator("main li")).to_contain_text("No events match these filters.").await.unwrap();
        page.evaluate::<(),bool>("() => { document.querySelector('main li').scrollIntoView({block:'center'}); return true; }",None).await.unwrap();
        capture(page,"audit-empty-320").await;
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn administration_polish_platform_frames() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| Box::pin(async move {
        let page=&journey.page;
        harness::sign_in(journey).await;
        journey.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await.unwrap();
        for (path,label) in [("users","platform-users"),("settings","platform-settings"),("support","platform-support"),("owner-recovery","platform-recovery")] {
            page.goto(&format!("{}/platform/{path}",journey.origin),None).await.unwrap();
            expect(page.locator(".platform-page")).to_be_visible().await.unwrap();
            assert_eq!(page.locator(".app-sidebar").count().await.unwrap(),0);
            for width in [1440,390,320] {
                page.set_viewport_size(Viewport {width,height:if width==1440 {1000}else{844}}).await.unwrap();
                capture(page,&format!("{label}-{width}")).await;
            }
        }
    })).await;
}
