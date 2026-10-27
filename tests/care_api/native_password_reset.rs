use super::*;
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};

pub(super) async fn queued_mail(app: &Application, email: &str, subject: &str) -> Vec<Value> {
    let query = Query::select()
        .column("task_data")
        .from("pg_loco_queue")
        .and_where(
            Func::cust("jsonb_extract_path_text")
                .args([Expr::col("task_data"), Expr::val("to")])
                .eq(email),
        )
        .and_where(
            Func::cust("jsonb_extract_path_text")
                .args([Expr::col("task_data"), Expr::val("subject")])
                .eq(subject),
        )
        .to_owned();
    app.fixture
        .admin
        .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get("", "task_data").unwrap())
        .collect()
}

#[tokio::test]
async fn native_password_reset_enrols_numeric_account_without_existing_password_and_rejects_replay()
{
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let delete = Query::delete()
        .from_table("auth_credentials")
        .and_where(Expr::col("user_id").eq(app.fixture.account.id))
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&delete))
        .await
        .unwrap();
    let sent = app
        .client
        .post(format!("{}/api/auth/request-password-reset", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":app.fixture.account.email,"redirectTo":"/reset-password/confirm"}))
        .send()
        .await
        .unwrap();
    let status = sent.status();
    let body = sent.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let messages = queued_mail(&app, &app.fixture.account.email, "Reset your password").await;
    assert_eq!(messages.len(), 1);
    let mail = &messages[0];
    let link = url::Url::parse(
        mail["text"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap(),
    )
    .unwrap();
    let token = link
        .path_segments()
        .unwrap()
        .next_back()
        .unwrap()
        .to_owned();
    let response = app
        .client
        .post(format!("{}/api/auth/reset-password", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"token":token,"newPassword":"New-native-reset-passphrase-42"}))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "credentials")
        .from("auth_credentials")
        .and_where(Expr::col("user_id").eq(app.fixture.account.id))
        .and_where(Expr::col("provider_id").eq("credential"))
        .and_where(Expr::col("password").is_not_null())
        .to_owned();
    let credential = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "credentials")
        .unwrap();
    assert_eq!(credential, 1);
    let replay = app
        .client
        .post(format!("{}/api/auth/reset-password", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"token":token,"newPassword":"Replayed-native-reset-passphrase-42"}))
        .send()
        .await
        .unwrap();
    assert!(replay.status().is_client_error());
    let login=app.client.post(format!("{}/api/auth/sign-in/email",app.origin)).header("origin",app.context.config.server.full_url()).json(&json!({"email":app.fixture.account.email.to_uppercase(),"password":"New-native-reset-passphrase-42"})).send().await.unwrap();
    let status = login.status();
    let body = login.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    app.close().await;
}
