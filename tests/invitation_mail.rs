#[path = "support/db.rs"]
mod db;
#[path = "support/fixtures.rs"]
pub mod fixtures;
#[path = "support/harness.rs"]
pub mod harness;
#[path = "support/template.rs"]
mod template;

use harness::named;
use loco_rs::bgworker::BackgroundWorker;
use med_tracker::models::care::invitations::delivery::InvitationMailWorker;
use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, Viewport};
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Address {
    address: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Message {
    from: Address,
    to: Vec<Address>,
    subject: String,
    text: String,
}

async fn delivery(db: &DatabaseConnection, recipient: &str) -> Option<(String, String, String)> {
    let query = Query::select()
        .columns(["status", "task_data"])
        .from("pg_loco_queue")
        .and_where(Expr::col("name").eq(InvitationMailWorker::class_name()))
        .and_where(
            Expr::expr(
                Func::cust("jsonb_extract_path_text")
                    .args([Expr::col("task_data"), Expr::val("to")]),
            )
            .eq(recipient),
        )
        .to_owned();
    let job = db
        .query_one_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()?;
    let payload: serde_json::Value = job.try_get("", "task_data").unwrap();
    let query = Query::select()
        .columns(["status", "category"])
        .from("invitation_mail_attempts")
        .and_where(Expr::col("id").eq(payload["attempt_id"].as_i64().unwrap()))
        .to_owned();
    let attempt = db
        .query_one_raw(db.get_database_backend().build(&query))
        .await
        .unwrap()?;
    Some((
        job.try_get("", "status").unwrap(),
        attempt.try_get("", "status").unwrap(),
        attempt
            .try_get::<Option<String>>("", "category")
            .unwrap()
            .unwrap_or_default(),
    ))
}

async fn wait_for_delivery(db: &DatabaseConnection, recipient: &str) -> (String, String, String) {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Some(state) = delivery(db, recipient).await
                && state.0 == "completed"
            {
                return state;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("invitation job did not complete")
}

async fn submit(journey: &harness::Journey, recipient: &str) {
    harness::sign_in(journey).await;
    journey
        .page
        .goto(
            &format!(
                "{}/households/persistence-fixture/admin/invitations",
                journey.origin
            ),
            None,
        )
        .await
        .unwrap();
    journey
        .page
        .get_by_label("Email", true)
        .fill(recipient, None)
        .await
        .unwrap();
    journey
        .page
        .get_by_role(AriaRole::Button, named("Send invitation"))
        .click(None)
        .await
        .unwrap();
    expect(journey.page.locator(".admin-invitations"))
        .to_contain_text(recipient)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "local SMTP acceptance runs through task test:invitation-mail"]
async fn invitation_smtp_capture_and_delivery_failure() {
    let capture = std::env::var("MEDTRACKER_MAILPIT_URL").expect("run task test:invitation-mail");
    let successful_capture = capture.clone();
    let evidence =
        std::path::Path::new("target/invitation-mail").join(uuid::Uuid::new_v4().to_string());
    let success_evidence = evidence.clone();
    let smtp_port = std::env::var("MEDTRACKER_SMTP_PORT")
        .unwrap()
        .parse()
        .unwrap();
    harness::smtp_journey(
        Viewport { width: 1440, height: 1000 },
        smtp_port,
        |journey| Box::pin(async move {
            let recipient = format!("invitation-{}@example.test", uuid::Uuid::new_v4());
            submit(journey, &recipient).await;
            let state = wait_for_delivery(&journey.admin, &recipient).await;
            assert_eq!(state, ("completed".into(), "sent_to_provider".into(), String::new()));
            let capture = successful_capture;
            let client = reqwest::Client::new();
            let listing: serde_json::Value = client.get(format!("{capture}/api/v1/messages"))
                .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
            let matching: Vec<_> = listing["messages"].as_array().unwrap().iter()
                .filter(|message| message["To"].as_array().unwrap().iter()
                    .any(|address| address["Address"] == recipient))
                .collect();
            assert_eq!(matching.len(), 1, "exactly one matching captured message");
            let id = matching[0]["ID"].as_str().unwrap();
            let message: Message = client.get(format!("{capture}/api/v1/message/{id}"))
                .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
            assert_eq!(message.from.address, "invitations@medtracker.test");
            assert_eq!(message.to.len(), 1);
            assert_eq!(message.to[0].address, recipient);
            assert_eq!(message.subject, "MedTracker household invitation");
            let link = message.text.split_whitespace().find(|word| word.starts_with("http://")).unwrap();
            let link = url::Url::parse(link).unwrap();
            assert_eq!(message.text.trim(), format!("You have been invited to a MedTracker household. Accept your invitation: {link}"));
            assert_eq!(link.origin().ascii_serialization(), journey.origin);
            assert_eq!(link.path(), "/invitations/accept");
            assert!(link.query_pairs().any(|(key, value)| key == "token" && !value.is_empty()));
            let preview = client.get(link.as_str()).send().await.unwrap();
            assert_eq!(preview.status(), reqwest::StatusCode::OK);
            let html = preview.text().await.unwrap();
            assert!(html.contains("Accept invitation"));
            assert!(html.contains("Synthetic household"));
            journey.page.goto(link.as_str(), None).await.unwrap();
            expect(journey.page.get_by_role(AriaRole::Heading, named("Accept invitation")))
                .to_be_visible().await.unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
            let listing: serde_json::Value = client.get(format!("{capture}/api/v1/messages"))
                .send().await.unwrap().json().await.unwrap();
            assert_eq!(listing["total"], 1, "no duplicate delivery after job completion");
            let raw = client.get(format!("{capture}/api/v1/message/{id}/raw"))
                .send().await.unwrap().error_for_status().unwrap().text().await.unwrap();
            std::fs::create_dir_all(&success_evidence).unwrap();
            std::fs::write(success_evidence.join("message.eml"), raw).unwrap();
            std::fs::write(success_evidence.join("message.json"), serde_json::to_vec_pretty(&message).unwrap()).unwrap();
            std::fs::write(success_evidence.join("delivery.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "scope":"local_smtp_capture", "job_status":state.0, "delivery_status":state.1,
                "matching_messages":1, "origin":journey.origin, "token_link_opened":true,
                "worker":"InvitationMailWorker -> Loco MailerWorker"
            })).unwrap()).unwrap();
            println!("Synthetic local SMTP evidence: {}", success_evidence.display());
            println!("Local SMTP capture: one message; sender, recipient, subject, body and local token link verified; job=completed; delivery=sent_to_provider");
        }),
    ).await;
    reqwest::Client::new()
        .put(format!("{capture}/api/v1/chaos"))
        .json(&serde_json::json!({"Sender":{"ErrorCode":421,"Probability":100}}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    harness::smtp_journey(
        Viewport { width: 1440, height: 1000 },
        smtp_port,
        |journey| Box::pin(async move {
            let recipient = format!("unavailable-{}@example.test", uuid::Uuid::new_v4());
            submit(journey, &recipient).await;
            let state = wait_for_delivery(&journey.admin, &recipient).await;
            assert_eq!(state, ("completed".into(), "failed".into(), "provider_unavailable".into()));
            let listing: serde_json::Value = reqwest::get(format!("{capture}/api/v1/messages"))
                .await.unwrap().error_for_status().unwrap().json().await.unwrap();
            assert!(!listing["messages"].as_array().unwrap().iter().any(|message|
                message["To"].as_array().unwrap().iter().any(|address| address["Address"] == recipient)));
            assert_eq!(listing["total"], 1);
            std::fs::write(evidence.join("unavailable.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "scope":"local_smtp_capture", "job_status":state.0, "delivery_status":state.1,
                "failure_category":state.2, "matching_messages":0, "smtp_error_code":421
            })).unwrap()).unwrap();
            println!("Local SMTP unavailable: job=completed; delivery=failed/provider_unavailable; no delivery success claimed");
        }),
    ).await;
}
