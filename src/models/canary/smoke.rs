use super::{invalid, text};
use loco_rs::app::AppContext;
use reqwest::{Client, Response, header};
use sea_orm::DbErr;
use std::{collections::BTreeMap, time::Duration};

fn cookies(response: &Response, jar: &mut BTreeMap<String, String>) -> Result<(), DbErr> {
    for cookie in response.headers().get_all(header::SET_COOKIE) {
        let cookie = cookie
            .to_str()
            .map_err(|_| invalid())?
            .split(';')
            .next()
            .ok_or_else(invalid)?;
        let (name, value) = cookie.split_once('=').ok_or_else(invalid)?;
        jar.insert(name.into(), value.into());
    }
    Ok(())
}
fn cookie_header(jar: &BTreeMap<String, String>) -> String {
    jar.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

pub(super) async fn run(ctx: &AppContext) -> Result<(), DbErr> {
    super::work(ctx).await.map_err(|_| invalid())?;
    let url = url::Url::parse(text(ctx, "smoke_url")?).map_err(|_| invalid())?;
    if url.scheme() != "http"
        || url.host_str() != Some("medtracker-canary.medtracker-canary.svc.cluster.local")
        || url.port() != Some(5150)
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid());
    }
    journey(ctx, &url).await
}

pub(super) async fn journey(ctx: &AppContext, url: &url::Url) -> Result<(), DbErr> {
    let origin = ctx.config.server.full_url();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "x-medtracker-canary-smoke",
        text(ctx, "smoke_key")?.parse().map_err(|_| invalid())?,
    );
    headers.insert(
        header::HOST,
        url::Url::parse(&origin)
            .map_err(|_| invalid())?
            .authority()
            .parse()
            .map_err(|_| invalid())?,
    );
    headers.insert(header::ORIGIN, origin.parse().map_err(|_| invalid())?);
    let client = Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| invalid())?;
    let mut jar = BTreeMap::new();
    let response = client
        .post(url.join("api/auth/sign-in/email").map_err(|_| invalid())?)
        .json(&serde_json::json!({"email":"persistence@example.test","password":"password"}))
        .send()
        .await
        .map_err(|_| invalid())?;
    if !response.status().is_success() {
        return Err(invalid());
    }
    cookies(&response, &mut jar)?;
    if jar.is_empty() {
        return Err(invalid());
    }
    for (path, expected) in [
        (
            "households/persistence-fixture/medications",
            "Synthetic tablets",
        ),
        ("households/canary-second/medications", "medication"),
    ] {
        let response = client
            .get(url.join(path).map_err(|_| invalid())?)
            .header(header::COOKIE, cookie_header(&jar))
            .send()
            .await
            .map_err(|_| invalid())?;
        if !response.status().is_success() {
            return Err(invalid());
        }
        cookies(&response, &mut jar)?;
        if !response
            .text()
            .await
            .map_err(|_| invalid())?
            .contains(expected)
        {
            return Err(invalid());
        }
    }
    let detail_url = url
        .join("households/persistence-fixture/medications/80001")
        .map_err(|_| invalid())?;
    let detail = client
        .get(detail_url.clone())
        .header(header::COOKIE, cookie_header(&jar))
        .send()
        .await
        .map_err(|_| invalid())?;
    if !detail.status().is_success() {
        return Err(invalid());
    }
    cookies(&detail, &mut jar)?;
    let html = detail.text().await.map_err(|_| invalid())?;
    let (mut form, before) = dose_form(&html)?;
    form.insert(
        "taken_at".into(),
        chrono::Utc::now()
            .with_timezone(&chrono_tz::Europe::London)
            .format("%Y-%m-%dT%H:%M")
            .to_string(),
    );
    let encoded = serde_urlencoded::to_string(&form).map_err(|_| invalid())?;
    let take_url = url
        .join("households/persistence-fixture/medications/80001/doses")
        .map_err(|_| invalid())?;
    for attempt in 0..2 {
        let response = client
            .post(take_url.clone())
            .header(header::COOKIE, cookie_header(&jar))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encoded.clone())
            .send()
            .await
            .map_err(|_| invalid())?;
        if response.status() != reqwest::StatusCode::SEE_OTHER {
            return Err(invalid());
        }
        cookies(&response, &mut jar)?;
        let response = client
            .get(detail_url.clone())
            .header(header::COOKIE, cookie_header(&jar))
            .send()
            .await
            .map_err(|_| invalid())?;
        if !response.status().is_success() {
            return Err(invalid());
        }
        cookies(&response, &mut jar)?;
        let after = stock(&response.text().await.map_err(|_| invalid())?)?;
        if after != before - sea_orm::prelude::Decimal::new(2, 0) {
            return Err(invalid());
        }
        tracing::info!(attempt, "Synthetic canary dose and stock replay passed");
    }
    let response = client
        .get(
            url.join("households/canary-unrelated/medications")
                .map_err(|_| invalid())?,
        )
        .header(header::COOKIE, cookie_header(&jar))
        .send()
        .await
        .map_err(|_| invalid())?;
    if response.status() != reqwest::StatusCode::FORBIDDEN {
        return Err(invalid());
    }
    Ok(())
}

fn stock(html: &str) -> Result<sea_orm::prelude::Decimal, DbErr> {
    let document = scraper::Html::parse_document(html);
    let selector =
        scraper::Selector::parse("[data-testid=current-supply]").map_err(|_| invalid())?;
    let text = document
        .select(&selector)
        .next()
        .ok_or_else(invalid)?
        .text()
        .collect::<String>();
    text.split_whitespace()
        .next()
        .ok_or_else(invalid)?
        .parse()
        .map_err(|_| invalid())
}
fn dose_form(html: &str) -> Result<(BTreeMap<String, String>, sea_orm::prelude::Decimal), DbErr> {
    let document = scraper::Html::parse_document(html);
    let forms = scraper::Selector::parse("form[data-dose-form]").map_err(|_| invalid())?;
    let inputs = scraper::Selector::parse("input[name]").map_err(|_| invalid())?;
    for form in document.select(&forms) {
        let values: BTreeMap<String, String> = form
            .select(&inputs)
            .filter_map(|input| {
                Some((
                    input.value().attr("name")?.into(),
                    input.value().attr("value").unwrap_or_default().into(),
                ))
            })
            .collect();
        if values.get("source_id").is_some_and(|v| v == "81001") {
            return Ok((values, stock(html)?));
        }
    }
    Err(invalid())
}
