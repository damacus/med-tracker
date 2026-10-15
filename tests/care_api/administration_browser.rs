use super::Application;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::json;

const SLUG: &str = "/households/persistence-fixture/admin/users";
type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

fn check(condition: bool, message: &str) -> TestResult<()> {
    if condition {
        Ok(())
    } else {
        Err(std::io::Error::other(message).into())
    }
}

fn check_eq<T: std::fmt::Debug + PartialEq>(
    actual: T,
    expected: T,
    message: &str,
) -> TestResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(
            std::io::Error::other(format!("{message}: expected {expected:?}, got {actual:?}"))
                .into(),
        )
    }
}

async fn browser_session(app: &Application, account_id: i64, email: &str) -> TestResult<String> {
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES({account_id},now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()"
        ))
        .await?;
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .ok_or_else(|| std::io::Error::other("identity service unavailable"))?;
    let mut request =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Post, "/sign-in/email");
    request
        .headers
        .insert("origin".into(), app.context.config.server.full_url());
    request
        .headers
        .insert("x-forwarded-for".into(), "127.0.0.1".into());
    request.body = Some(serde_json::to_vec(
        &json!({"email":email,"password":"password"}),
    )?);
    let response = med_tracker::models::identity::better_auth::dispatch(
        &service,
        request,
        format!("administration-browser-{account_id}"),
    )
    .await?;
    check_eq(response.status, 200, "browser sign-in status")?;
    Ok(response
        .headers
        .get("set-cookie")
        .ok_or_else(|| std::io::Error::other("browser sign-in cookie missing"))?
        .split(';')
        .next()
        .ok_or_else(|| std::io::Error::other("browser sign-in cookie malformed"))?
        .to_owned())
}

fn cookies(previous: &str, response: &reqwest::Response) -> String {
    let mut cookies = previous.to_owned();
    for value in response.headers().get_all("set-cookie") {
        if let Some(pair) = value
            .to_str()
            .ok()
            .and_then(|value| value.split(';').next())
            && !pair.ends_with('=')
        {
            cookies.push_str("; ");
            cookies.push_str(pair);
        }
    }
    cookies
}

async fn page(
    app: &Application,
    client: &reqwest::Client,
    cookie: &str,
    path: &str,
) -> TestResult<(reqwest::StatusCode, String, String)> {
    let response = client
        .get(format!("{}{path}", app.origin))
        .header("cookie", cookie)
        .send()
        .await?;
    let next_cookie = cookies(cookie, &response);
    let status = response.status();
    let body = response.text().await?;
    Ok((status, body, next_cookie))
}

fn authenticity_token(body: &str) -> TestResult<String> {
    Ok(body
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .filter(|token| !token.is_empty())
        .ok_or_else(|| std::io::Error::other("form authenticity token missing"))?
        .to_owned())
}

async fn post_role(
    app: &Application,
    client: &reqwest::Client,
    cookie: &str,
    membership_id: i64,
    role: &str,
    token: &str,
) -> TestResult<reqwest::Response> {
    let public = app.context.config.server.full_url();
    let url = url::Url::parse(&public)?;
    let authority = match url.port() {
        Some(port) => format!(
            "{}:{port}",
            url.host_str()
                .ok_or_else(|| std::io::Error::other("public URL host missing"))?
        ),
        None => url
            .host_str()
            .ok_or_else(|| std::io::Error::other("public URL host missing"))?
            .to_owned(),
    };
    let mut form = url::form_urlencoded::Serializer::new(String::new());
    form.append_pair("authenticity_token", token);
    form.append_pair("role", role);
    client
        .post(format!(
            "{}{SLUG}/{membership_id}/membership_role",
            app.origin
        ))
        .header("cookie", cookie)
        .header("host", authority)
        .header("origin", public.trim_end_matches('/'))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(form.finish())
        .send()
        .await
        .map_err(Into::into)
}

async fn membership_state(app: &Application, id: i64) -> TestResult<(String, i64)> {
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT role,permissions_version::bigint AS version FROM household_memberships WHERE id=$1",
            [id.into()],
        ))
        .await?
        .ok_or_else(|| std::io::Error::other(format!("membership {id} missing")))?;
    Ok((row.try_get("", "role")?, row.try_get("", "version")?))
}

#[tokio::test]
async fn administration_browser_manager_edits_membership_not_account_or_person() -> TestResult<()> {
    let app = Application::new().await;
    let result: TestResult<()> = async {
        app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO people(id,clinical_name,person_type,has_capacity,created_at,updated_at) VALUES
             (73004,'Synthetic target',0,true,now(),now()),
             (73005,'Accountless participant',0,true,now(),now()),
             (73006,'Ordinary participant',0,true,now(),now()),
             (73007,'Foreign participant',0,true,now(),now()),
             (73008,'Inactive participant',0,true,now(),now());
             INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES
             (71002,73004,'Synthetic target','target-member@example.test',2,now(),now()),
             (71003,73006,'Ordinary participant','ordinary-member@example.test',2,now(),now());
             INSERT INTO identity_provider_accounts(id,account_id,provider_id,provider_account_id,password,created_at,updated_at) VALUES
             ('administration-target-credential',71002,'credential','71002',crypt('password',gen_salt('bf',4)),now(),now()),
             ('administration-ordinary-credential',71003,'credential','71003',crypt('password',gen_salt('bf',4)),now(),now());
             INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES
             (72002,71002,'Foreign household','foreign-administration-fixture','Europe/London',now(),now());
             INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES
             (74002,72001,73004,'member','active',now(),now(),now()),
             (74003,72001,73005,'member','active',now(),now(),now()),
             (74004,72001,73006,'member','active',now(),now(),now()),
             (74005,72002,73007,'member','active',now(),now(),now()),
             (74006,72001,73008,'member','suspended',now(),now(),now())",
        )
        .await?;
        let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
        let manager_cookie = browser_session(&app, 71001, "persistence@example.test").await?;
        let before = membership_state(&app, 74002).await?;
        let manager_before = membership_state(&app, 74001).await?;
        let accountless_before = membership_state(&app, 74003).await?;
        let foreign_before = membership_state(&app, 74005).await?;

        let (status, users, manager_cookie) = page(&app, &client, &manager_cookie, SLUG).await?;
        check_eq(status, reqwest::StatusCode::OK, "manager users status")?;
        for expected in ["Synthetic target", "Accountless participant", "/admin/users/74002/edit", "/admin/users/74003/edit"] {
            check(users.contains(expected), &format!("manager users missing {expected}"))?;
        }
        for absent in ["/admin/users/71002/edit", "/admin/users/73004/edit", "/admin/users/74005/edit", "/admin/users/74006/edit", "target-member@example.test", "ordinary-member@example.test"] {
            check(!users.contains(absent), &format!("manager users exposed {absent}"))?;
        }

        let (status, edit, manager_cookie) =
            page(&app, &client, &manager_cookie, &format!("{SLUG}/74002/edit")).await?;
        check_eq(status, reqwest::StatusCode::OK, "member edit status")?;
        check(edit.contains(&format!("action=\"{SLUG}/74002/membership_role\"")), "member form action must use membership id")?;
        check(edit.contains("value=\"member\" selected"), "member role must be preselected")?;
        let token = authenticity_token(&edit)?;
        let changed = post_role(&app, &client, &manager_cookie, 74002, "administrator", &token).await?;
        check_eq(changed.status(), reqwest::StatusCode::SEE_OTHER, "valid role response")?;
        check_eq(
            changed.headers().get("location").and_then(|value| value.to_str().ok()),
            Some("/households/persistence-fixture/admin/users"),
            "valid role redirect",
        )?;
        let manager_cookie = cookies(&manager_cookie, &changed);
        let after = membership_state(&app, 74002).await?;
        check_eq(after.0.as_str(), "administrator", "changed target role")?;
        check(after.1 > before.1, "changed target permissions version did not increase")?;
        check_eq(membership_state(&app, 74001).await?, manager_before, "manager membership changed")?;
        check_eq(membership_state(&app, 74003).await?, accountless_before, "accountless membership changed")?;

        let (status, refreshed, manager_cookie) =
            page(&app, &client, &manager_cookie, &format!("{SLUG}/74002/edit")).await?;
        check_eq(status, reqwest::StatusCode::OK, "refreshed member edit status")?;
        check(refreshed.contains("value=\"administrator\" selected"), "refreshed role does not show administrator")?;
        let invalid = post_role(
            &app, &client, &manager_cookie, 74002, "invalid-role", &authenticity_token(&refreshed)?,
        ).await?;
        check_eq(invalid.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY, "invalid role response")?;
        let invalid_body = invalid.text().await?;
        check(invalid_body.contains("role=\"alert\""), "invalid role error alert missing")?;
        check(invalid_body.contains("aria-invalid=\"true\""), "invalid role field state missing")?;
        check_eq(membership_state(&app, 74002).await?, after.clone(), "invalid role changed membership")?;

        let (status, accountless, manager_cookie) =
            page(&app, &client, &manager_cookie, &format!("{SLUG}/74003/edit")).await?;
        check_eq(status, reqwest::StatusCode::OK, "accountless member edit status")?;
        check(accountless.contains("Accountless participant"), "accountless member name missing")?;
        check(accountless.contains(&format!("action=\"{SLUG}/74003/membership_role\"")), "accountless form action must use membership id")?;
        let (status, _, manager_cookie) =
            page(&app, &client, &manager_cookie, &format!("{SLUG}/74005/edit")).await?;
        check_eq(status, reqwest::StatusCode::NOT_FOUND, "foreign member edit status")?;
        let foreign = post_role(
            &app, &client, &manager_cookie, 74005, "administrator", &authenticity_token(&accountless)?,
        ).await?;
        check(matches!(foreign.status(), reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::FORBIDDEN), "foreign membership POST was accepted")?;
        check_eq(membership_state(&app, 74005).await?, foreign_before, "foreign membership changed")?;

        let ordinary_cookie = browser_session(&app, 71003, "ordinary-member@example.test").await?;
        let (status, _, ordinary_cookie) = page(&app, &client, &ordinary_cookie, SLUG).await?;
        check_eq(status, reqwest::StatusCode::FORBIDDEN, "ordinary member users status")?;
        let (status, profile, ordinary_cookie) = page(&app, &client, &ordinary_cookie, "/profile").await?;
        check_eq(status, reqwest::StatusCode::OK, "ordinary member profile status")?;
        let denied = post_role(
            &app, &client, &ordinary_cookie, 74002, "member", &authenticity_token(&profile)?,
        ).await?;
        check_eq(denied.status(), reqwest::StatusCode::FORBIDDEN, "ordinary member role POST status")?;
        check_eq(membership_state(&app, 74002).await?, after, "ordinary member changed target role")?;
        Ok(())
    }.await;
    app.close().await;
    result
}
