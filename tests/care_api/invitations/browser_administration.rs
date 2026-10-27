// SeaQuery has no PostgreSQL trigger/function DDL API. The exclusive selection
// failure probes retain narrow DDL and remove it before closing the App.

use super::*;
use med_tracker::models::_entities::{household_invitations, household_memberships};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbErr, EntityTrait, Order, QueryFilter, Set,
    sea_query::{Alias, Asterisk, Expr, ExprTrait, Func, Query},
};
use sha2::{Digest, Sha256};
use std::{error::Error, io};

fn invitation_document(html: &str) -> Result<(Vec<String>, Option<String>), Box<dyn Error>> {
    let document = scraper::Html::parse_document(html);
    let rows = scraper::Selector::parse("li.card h3")
        .map_err(|_| io::Error::other("invalid invitation row selector"))?;
    let next = scraper::Selector::parse("a[rel='next']")
        .map_err(|_| io::Error::other("invalid invitation next selector"))?;
    Ok((
        document
            .select(&rows)
            .map(|row| row.text().collect::<String>().trim().to_owned())
            .collect(),
        document
            .select(&next)
            .next()
            .and_then(|link| link.value().attr("href"))
            .map(str::to_owned),
    ))
}

#[tokio::test]
async fn invitation_browser_pages_fifty_stably_and_preserves_locale() -> Result<(), Box<dyn Error>>
{
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let cookie = super::super::platform::sign_in(
            &app,
            app.fixture.account.id,
            app.fixture.account.email.as_str(),
        )
        .await;
        for index in 1..=101 {
            let created = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            household_invitations::ActiveModel {
                household_id: Set(app.fixture.household.id),
                email: Set(page_email(&app, index)),
                membership_role: Set("member".into()),
                token_digest: Set(uuid::Uuid::new_v4().to_string()),
                invited_by_membership_id: Set(app.fixture.membership.id),
                expires_at: Set(chrono::Utc::now().naive_utc() + chrono::Duration::days(7)),
                created_at: Set(created),
                updated_at: Set(created),
                ..Default::default()
            }
            .insert(&app.fixture.admin)
            .await?;
        }
        let first_url = format!(
            "{}/households/{generated_slug}/admin/invitations?lang=es",
            app.origin,
            generated_slug = app.fixture.household.slug
        );
        let mut destination = first_url.clone();
        let mut seen = Vec::new();
        for expected_count in [50, 50, 1] {
            let response = app
                .client
                .get(&destination)
                .header("cookie", &cookie)
                .header("accept-language", "cy")
                .send()
                .await?;
            assert_eq!(response.status(), reqwest::StatusCode::OK);
            assert_eq!(
                response
                    .headers()
                    .get("cache-control")
                    .and_then(|value| value.to_str().ok()),
                Some("no-store")
            );
            let body = response.text().await?;
            assert!(body.contains("lang=\"es\""));
            let (emails, next) = invitation_document(&body)?;
            assert_eq!(
                emails.len(),
                expected_count,
                "invitation page must be bounded"
            );
            seen.extend(emails);
            if expected_count == 1 {
                assert!(next.is_none());
            } else {
                let next = next.ok_or_else(|| io::Error::other("missing next invitation page"))?;
                let next = url::Url::parse(&destination)?.join(&next)?;
                assert_eq!(next.origin(), url::Url::parse(&app.origin)?.origin());
                assert_eq!(
                    next.path(),
                    &format!(
                        "/households/{generated_slug}/admin/invitations",
                        generated_slug = app.fixture.household.slug
                    )
                );
                assert!(
                    next.query_pairs()
                        .any(|(key, value)| key == "lang" && value == "es")
                );
                destination = next.to_string();
            }
        }
        let expected = (1..=101)
            .rev()
            .map(|number| page_email(&app, number))
            .collect::<Vec<_>>();
        assert_eq!(seen, expected);
        let repeated = app
            .client
            .get(first_url)
            .header("cookie", &cookie)
            .send()
            .await?
            .text()
            .await?;
        assert_eq!(
            invitation_document(&repeated)?.0,
            expected.into_iter().take(50).collect::<Vec<_>>()
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

async fn select_first_household(app: &Application) -> Result<String, Box<dyn Error>> {
    let cookie = super::super::platform::sign_in(
        app,
        app.fixture.account.id,
        app.fixture.account.email.as_str(),
    )
    .await;
    let (cookie, html) =
        super::super::platform::form_page(app, &cookie, "/households/choose").await;
    let csrf = super::super::platform::hidden_field(&html, "authenticity_token");
    if csrf.is_empty() {
        return Err(io::Error::other("household selection token missing").into());
    }
    let response = super::super::platform::post_form(
        app,
        &cookie,
        "/households/select",
        &[
            (
                "household_id",
                app.fixture.household.id.to_string().as_str(),
            ),
            ("authenticity_token", &csrf),
        ],
    )
    .await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response.url().path(),
        &format!(
            "/households/{generated_slug}/dashboard",
            generated_slug = app.fixture.household.slug
        )
    );
    Ok(super::super::platform::response_cookies(&cookie, &response))
}

async fn session_households(app: &Application) -> Result<Vec<(i64, i64)>, DbErr> {
    app.fixture
        .admin
        .query_all_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col("id"))
                    .expr(Expr::col("active_household_id"))
                    .from("identity_sessions")
                    .and_where(
                        Expr::col("account_id")
                            .eq(Expr::val(app.fixture.account.id))
                            .and(Expr::col("active"))
                            .and(Expr::col("purpose").eq(Expr::val("authenticated"))),
                    )
                    .order_by("id", Order::Asc)
                    .to_owned(),
            ),
        )
        .await?
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<i64>("", "id")?,
                row.try_get::<i64>("", "active_household_id")?,
            ))
        })
        .collect()
}

#[tokio::test]
async fn invitation_acceptance_pages_keep_locale_and_safe_invalid_link_recovery()
-> Result<(), Box<dyn Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let acceptance = super::seed_acceptance(&app).await;
        let cookie = super::super::platform::sign_in(
            &app,
            app.fixture.account.id,
            app.fixture.account.email.as_str(),
        )
        .await;
        for (locale, heading) in [
            ("en", "Accept invitation"),
            ("cy", "Derbyn gwahoddiad"),
            ("ga", "Glac le cuireadh"),
            ("pt", "Aceitar convite"),
            ("es", "Aceptar invitación"),
        ] {
            let response = app
                .client
                .get(format!(
                    "{}/invitations/accept?token={generated_acceptance_token}&lang={locale}",
                    app.origin,
                    generated_acceptance_token = acceptance.token
                ))
                .header("accept-language", "cy")
                .header("cookie", &cookie)
                .send()
                .await?;
            assert_eq!(response.status(), reqwest::StatusCode::OK, "{locale}");
            assert_eq!(
                response
                    .headers()
                    .get("cache-control")
                    .and_then(|value| value.to_str().ok()),
                Some("no-store")
            );
            let body = response.text().await?;
            assert!(
                body.contains(&format!("<html lang=\"{locale}\"")),
                "document language {locale}"
            );
            assert!(body.contains(heading), "invitation heading {locale}");
            assert!(
                body.contains(&format!("name=\"lang\" value=\"{locale}\"")),
                "form language {locale}"
            );
        }
        for query in [
            "lang=es",
            &format!(
                "token={generated_acceptance_token}&lang=es&lang=en",
                generated_acceptance_token = acceptance.token
            ),
            &format!(
                "token={generated_acceptance_token}&token=other&lang=es",
                generated_acceptance_token = acceptance.token
            ),
            &format!(
                "token={generated_acceptance_token}&lang=xx",
                generated_acceptance_token = acceptance.token
            ),
            &format!(
                "token={generated_acceptance_token}&lang=es&next=https%3A%2F%2Fevil.example.test",
                generated_acceptance_token = acceptance.token
            ),
        ] {
            let response = app
                .client
                .get(format!("{}/invitations/accept?{query}", app.origin))
                .header("accept-language", "es")
                .send()
                .await?;
            assert_eq!(
                response.status(),
                reqwest::StatusCode::UNPROCESSABLE_ENTITY,
                "{query}"
            );
            assert_eq!(
                response
                    .headers()
                    .get("cache-control")
                    .and_then(|value| value.to_str().ok()),
                Some("no-store")
            );
            assert!(
                response
                    .headers()
                    .get("content-type")
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|value| value.starts_with("text/html"))
            );
            let body = response.text().await?;
            let document = scraper::Html::parse_document(&body);
            let recovery = scraper::Selector::parse("section.card a.btn[href='/']")
                .map_err(|_| io::Error::other("invalid recovery selector"))?;
            assert!(
                document.select(&recovery).next().is_some(),
                "local recovery {query}"
            );
            assert!(
                !body.contains("evil.example.test"),
                "query must not be reflected"
            );
        }
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_acceptance_selects_only_the_current_session() -> Result<(), Box<dyn Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let acceptance = super::seed_acceptance(&app).await;
        let first = select_first_household(&app).await?;
        let second = select_first_household(&app).await?;
        let before = session_households(&app).await?;
        assert_eq!(before.len(), 2);
        assert!(
            before
                .iter()
                .all(|(_, household)| *household == app.fixture.household.id)
        );
        let (first, preview) = super::super::platform::form_page(
            &app,
            &first,
            &format!(
                "/invitations/accept?token={generated_acceptance_token}",
                generated_acceptance_token = acceptance.token
            ),
        )
        .await;
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        if csrf.is_empty() {
            return Err(io::Error::other("invitation acceptance token missing").into());
        }
        let response = super::super::platform::post_form(
            &app,
            &first,
            "/invitations/accept",
            &[
                ("token", acceptance.token.as_str()),
                ("authenticity_token", &csrf),
            ],
        )
        .await;
        let accepted_status = response.status();
        let accepted_path = response.url().path().to_owned();
        let first = super::super::platform::response_cookies(&first, &response);
        let after = session_households(&app).await?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        let mut destinations = Vec::new();
        for cookie in [&first, &second] {
            let response = client
                .get(format!("{}/", app.origin))
                .header("cookie", cookie)
                .send()
                .await?;
            destinations.push((
                response.status(),
                response
                    .headers()
                    .get("location")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned),
            ));
        }
        let navigation = app
            .client
            .get(format!(
                "{}/households/{generated_target_slug}/profile/navigation",
                app.origin,
                generated_target_slug = acceptance.target_slug
            ))
            .header("cookie", &first)
            .send()
            .await?;
        let navigation_status = navigation.status();
        let navigation: Value = navigation.json().await?;
        let state = app
            .fixture
            .admin
            .query_one_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::select()
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col("person_id"))
                                    .from("accounts")
                                    .and_where(
                                        Expr::col("id").eq(Expr::val(app.fixture.account.id)),
                                    )
                                    .to_owned(),
                            ),
                            "person_id",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("people")
                                    .and_where(Expr::col("id").eq(Expr::val(app.fixture.adult.id)))
                                    .to_owned(),
                            ),
                            "people",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("household_memberships")
                                    .and_where(
                                        Expr::col("household_id")
                                            .eq(Expr::val(acceptance.target_household.id))
                                            .and(
                                                Expr::col("person_id")
                                                    .eq(Expr::val(app.fixture.adult.id)),
                                            ),
                                    )
                                    .to_owned(),
                            ),
                            "memberships",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("person_access_grants")
                                    .and_where(
                                        Expr::col("household_id")
                                            .eq(Expr::val(acceptance.target_household.id))
                                            .and(
                                                Expr::col("person_id")
                                                    .eq(Expr::val(app.fixture.adult.id)),
                                            ),
                                    )
                                    .to_owned(),
                            ),
                            "self_grants",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("person_access_grants")
                                    .and_where(
                                        Expr::col("household_id")
                                            .eq(Expr::val(acceptance.target_household.id))
                                            .and(
                                                Expr::col("person_id")
                                                    .eq(Expr::val(acceptance.invited_child.id)),
                                            ),
                                    )
                                    .to_owned(),
                            ),
                            "dependent_grants",
                        )
                        .expr_as(
                            Expr::from(
                                Query::select()
                                    .expr(Expr::col(Asterisk).count())
                                    .from("carer_relationships")
                                    .and_where(
                                        Expr::col("household_id")
                                            .eq(Expr::val(acceptance.target_household.id)),
                                    )
                                    .to_owned(),
                            ),
                            "relationships",
                        )
                        .to_owned(),
                ),
            )
            .await?
            .ok_or_else(|| io::Error::other("accepted identity state missing"))?;
        assert_eq!(state.try_get::<i64>("", "person_id")?, app.fixture.adult.id);
        for (field, count) in [
            ("people", 1),
            ("memberships", 1),
            ("self_grants", 0),
            ("dependent_grants", 0),
            ("relationships", 0),
        ] {
            assert_eq!(state.try_get::<i64>("", field)?, count, "canonical {field}");
        }
        assert_eq!(accepted_status, reqwest::StatusCode::OK);
        assert_eq!(
            after
                .iter()
                .filter(|(_, household)| *household == acceptance.target_household.id)
                .count(),
            1,
            "acceptance must select the joined household for exactly the current session"
        );
        assert_eq!(after.len(), 2);
        assert_eq!(
            before.iter().map(|(id, _)| id).collect::<Vec<_>>(),
            after.iter().map(|(id, _)| id).collect::<Vec<_>>()
        );
        assert_eq!(
            after
                .iter()
                .filter(|(_, household)| *household == app.fixture.household.id)
                .count(),
            1
        );
        assert_eq!(
            accepted_path,
            format!(
                "/households/{generated_target_slug}/dashboard",
                generated_target_slug = acceptance.target_slug
            )
        );
        assert_eq!(
            destinations,
            vec![
                (
                    reqwest::StatusCode::SEE_OTHER,
                    Some(format!(
                        "/households/{generated_target_slug}/dashboard",
                        generated_target_slug = acceptance.target_slug
                    ))
                ),
                (
                    reqwest::StatusCode::SEE_OTHER,
                    Some(format!(
                        "/households/{generated_slug}/dashboard",
                        generated_slug = app.fixture.household.slug
                    ))
                ),
            ]
        );
        assert_eq!(navigation_status, reqwest::StatusCode::OK);
        assert_eq!(
            navigation["shell"]["active_household_id"],
            acceptance.target_household.id.to_string().as_str()
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

fn invitation_row(document: &scraper::Html, email: &str) -> Result<String, Box<dyn Error>> {
    let selector = scraper::Selector::parse("li.card")
        .map_err(|_| io::Error::other("invalid invitation card selector"))?;
    document
        .select(&selector)
        .find(|row| row.text().any(|text| text.contains(email)))
        .map(|row| row.html())
        .ok_or_else(|| io::Error::other(format!("missing invitation card for {email}")).into())
}

async fn assert_forbidden_presentation(response: reqwest::Response) -> Result<(), Box<dyn Error>> {
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html")),
        "forbidden invitation response must render an HTML recovery page"
    );
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    let body = response.text().await?;
    assert!(body.contains("<html lang=\"es\""));
    assert!(body.contains("Ya no tienes acceso a esta página de administración."));
    assert!(body.contains("Vuelve al panel para revisar tu acceso actual al hogar."));
    let document = scraper::Html::parse_document(&body);
    let recovery = scraper::Selector::parse("main section a.btn")
        .map_err(|_| io::Error::other("invalid forbidden recovery-link selector"))?;
    let link = document
        .select(&recovery)
        .next()
        .ok_or_else(|| io::Error::other("missing forbidden recovery link"))?;
    assert_eq!(link.value().attr("href"), Some("/"));
    Ok(())
}

#[tokio::test]
async fn invitation_browser_empty_and_terminal_states_expose_only_permitted_actions()
-> Result<(), Box<dyn Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let cookie = super::super::platform::sign_in(
            &app,
            app.fixture.account.id,
            app.fixture.account.email.as_str(),
        )
        .await;

        let path = &format!(
            "/households/{generated_slug}/admin/invitations?lang=es",
            generated_slug = app.fixture.household.slug
        );
        let empty = app
            .client
            .get(format!("{}{path}", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(empty.status(), reqwest::StatusCode::OK);
        let empty = empty.text().await?;
        assert!(empty.contains("<html lang=\"es\""));
        assert!(empty.contains("Invitaciones"));
        assert!(
            empty.contains("No hay invitaciones"),
            "translated empty state"
        );
        let pending =
            local_invitation(&app, "pending-c4@example.test", "member", 7, 0, "pending").await?;
        let expired = local_invitation(
            &app,
            "expired-c4@example.test",
            "administrator",
            -1,
            2,
            "pending",
        )
        .await?;
        let accepted =
            local_invitation(&app, "accepted-c4@example.test", "member", 7, 3, "accepted").await?;
        let revoked =
            local_invitation(&app, "revoked-c4@example.test", "member", 7, 4, "revoked").await?;
        let response = app
            .client
            .get(format!("{}{path}", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await?;
        let document = scraper::Html::parse_document(&body);
        for (email, label, actionable) in [
            (pending.email.as_str(), "Pendiente", true),
            (expired.email.as_str(), "Caducada", true),
            (accepted.email.as_str(), "Aceptada", false),
            (revoked.email.as_str(), "Revocada", false),
        ] {
            let row = invitation_row(&document, email)?;
            assert!(row.contains(label), "translated status for {email}");
            assert_eq!(row.contains("/resend"), actionable, "resend for {email}");
            assert_eq!(row.contains("/cancel"), actionable, "cancel for {email}");
        }
        assert!(body.contains("name=\"lang\" value=\"es\""));
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_browser_locales_preserve_forms_feedback_and_recovery()
-> Result<(), Box<dyn Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let cookie = super::super::platform::sign_in(
            &app,
            app.fixture.account.id,
            app.fixture.account.email.as_str(),
        )
        .await;
        let path = &format!(
            "/households/{generated_slug}/admin/invitations",
            generated_slug = app.fixture.household.slug
        );
        for (locale, heading) in [
            ("en", "Invitations"),
            ("cy", "Gwahoddiadau"),
            ("ga", "Cuirí"),
            ("pt", "Convites"),
            ("es", "Invitaciones"),
        ] {
            let response = app
                .client
                .get(format!("{}{path}?lang={locale}", app.origin))
                .header("cookie", &cookie)
                .header("accept-language", "cy")
                .send()
                .await?;
            assert_eq!(response.status(), reqwest::StatusCode::OK, "{locale}");
            let body = response.text().await?;
            assert!(
                body.contains(&format!("<html lang=\"{locale}\"")),
                "{locale}"
            );
            assert!(body.contains(heading), "translated title {locale}");
            assert!(
                body.contains(&format!("name=\"lang\" value=\"{locale}\"")),
                "form locale {locale}"
            );
        }
        let (cookie, form) =
            super::super::platform::form_page(&app, &cookie, &format!("{path}?lang=es")).await;
        let csrf = super::super::platform::hidden_field(&form, "authenticity_token");
        let before = manager_effect_counts(&app).await?;
        let invalid = super::super::platform::post_form(
            &app,
            &cookie,
            path,
            &[
                ("authenticity_token", &csrf),
                ("lang", "es"),
                ("email", "invalid-email"),
                ("membership_role", "member"),
            ],
        )
        .await;
        assert_eq!(invalid.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        let invalid = invalid.text().await?;
        assert!(invalid.contains("invalid-email"), "draft preserved");
        assert!(invalid.contains("<html lang=\"es\""));
        let document = scraper::Html::parse_document(&invalid);
        let field_error = scraper::Selector::parse("#form-error-email")
            .map_err(|_| io::Error::other("invalid email error selector"))?;
        let message = document
            .select(&field_error)
            .next()
            .ok_or_else(|| io::Error::other("missing email validation message"))?
            .text()
            .collect::<String>();
        assert!(
            message.contains("correo electrónico") && message.contains("válido"),
            "translated invalid-email message: {message}"
        );
        assert!(!message.contains("Email is invalid"));
        let rejected = super::super::platform::post_form(
            &app,
            &cookie,
            path,
            &[
                ("authenticity_token", &csrf),
                ("lang", "es"),
                (
                    "email",
                    &browser_email(&app, "retired-care-c4@example.test"),
                ),
                ("membership_role", "member"),
                ("relationship_type", "parent"),
                ("access_level", "manage"),
                ("dependent_ids[]", app.fixture.minor.id.to_string().as_str()),
            ],
        )
        .await;
        assert_eq!(rejected.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        let rejected = rejected.text().await?;
        let document = scraper::Html::parse_document(&rejected);
        let base_error = scraper::Selector::parse("#form-error-invitation-form")
            .map_err(|_| io::Error::other("invalid form error selector"))?;
        let message = document
            .select(&base_error)
            .next()
            .ok_or_else(|| io::Error::other("missing retired-field validation message"))?
            .text()
            .collect::<String>();
        assert!(
            message.contains("autoridad de cuidado"),
            "translated retired-field error: {message}"
        );
        assert!(!message.contains("Household invitations cannot assign care authority"));
        assert_eq!(
            manager_effect_counts(&app).await?,
            before,
            "invalid forms cannot create invitation, mail, version, audit or grant effects"
        );
        let created = super::super::platform::post_form(
            &app,
            &cookie,
            path,
            &[
                ("authenticity_token", &csrf),
                ("lang", "es"),
                ("email", &browser_email(&app, "locale-c4@example.test")),
                ("membership_role", "member"),
            ],
        )
        .await;
        assert_eq!(created.status(), reqwest::StatusCode::OK);
        assert!(
            created
                .url()
                .query()
                .is_some_and(|query| query.contains("lang=es"))
        );
        let created = created.text().await?;
        assert!(created.contains(&browser_email(&app, "locale-c4@example.test")));
        assert!(created.contains("en cola"), "queued delivery feedback");
        let refreshed = app
            .client
            .get(format!("{}{path}?lang=es", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?
            .text()
            .await?;
        assert!(refreshed.contains(&browser_email(&app, "locale-c4@example.test")));
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_browser_stale_actions_recheck_manager_and_invitation_state()
-> Result<(), Box<dyn Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let acceptance = super::seed_acceptance(&app).await;
        let cookie = super::super::platform::sign_in(
            &app,
            app.fixture.account.id,
            app.fixture.account.email.as_str(),
        )
        .await;
        let path = &format!(
            "/households/{generated_slug}/admin/invitations",
            generated_slug = app.fixture.household.slug
        );
        let (cookie, html) =
            super::super::platform::form_page(&app, &cookie, &format!("{path}?lang=es")).await;
        let csrf = super::super::platform::hidden_field(&html, "authenticity_token");
        let foreign = app
            .client
            .get(format!(
                "{}/households/{generated_target_slug}/admin/invitations?lang=es",
                app.origin,
                generated_target_slug = acceptance.target_slug
            ))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_forbidden_presentation(foreign).await?;
        let stale =
            local_invitation(&app, "stale-c4@example.test", "member", 7, 0, "accepted").await?;
        for action in ["resend", "cancel"] {
            let response = super::super::platform::post_form(
                &app,
                &cookie,
                &format!(
                    "{path}/{generated_87111}/{action}",
                    generated_87111 = stale.id
                ),
                &[("authenticity_token", &csrf), ("lang", "es")],
            )
            .await;
            assert_eq!(
                response.status(),
                reqwest::StatusCode::UNPROCESSABLE_ENTITY,
                "accepted invitation {action}"
            );
        }
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("household_memberships")
                        .value("role", Expr::val("member"))
                        .and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id)))
                        .to_owned(),
                ),
            )
            .await?;
        let denied = super::super::platform::post_form(
            &app,
            &cookie,
            path,
            &[
                ("authenticity_token", &csrf),
                ("lang", "es"),
                (
                    "email",
                    &browser_email(&app, "lost-authority-c4@example.test"),
                ),
                ("membership_role", "member"),
            ],
        )
        .await;
        assert_forbidden_presentation(denied).await?;
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("household_memberships")
                        .value("status", Expr::val("revoked"))
                        .value("revoked_at", Expr::expr(Func::cust("now")))
                        .and_where(Expr::col("id").eq(Expr::val(app.fixture.membership.id)))
                        .to_owned(),
                ),
            )
            .await?;
        let before = manager_effect_counts(&app).await?;
        let listed = app
            .client
            .get(format!("{}{path}?lang=es", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_forbidden_presentation(listed).await?;
        for (action, endpoint) in [
            ("create", path.to_owned()),
            (
                "resend",
                format!(
                    "{path}/{generated_87111}/resend",
                    generated_87111 = stale.id
                ),
            ),
            (
                "cancel",
                format!(
                    "{path}/{generated_87111}/cancel",
                    generated_87111 = stale.id
                ),
            ),
        ] {
            let response = super::super::platform::post_form(
                &app,
                &cookie,
                &endpoint,
                &[
                    ("authenticity_token", &csrf),
                    ("lang", "es"),
                    (
                        "email",
                        &browser_email(&app, "revoked-manager-c4@example.test"),
                    ),
                    ("membership_role", "member"),
                ],
            )
            .await;
            assert_forbidden_presentation(response)
                .await
                .map_err(|error| io::Error::other(format!("revoked manager {action}: {error}")))?;
        }
        assert_eq!(
            manager_effect_counts(&app).await?,
            before,
            "revoked manager has no invitation, mail, version, audit or grant effects"
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

async fn manager_effect_counts(app: &Application) -> Result<Vec<i64>, DbErr> {
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from("household_invitations")
                                .and_where(
                                    Expr::col("household_id")
                                        .eq(Expr::val(app.fixture.household.id)),
                                )
                                .to_owned(),
                        ),
                        "invitations",
                    )
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from("pg_loco_queue")
                                .and_where(
                                    Expr::expr(
                                        Func::cust("jsonb_extract_path_text").args([
                                            Expr::col("task_data"),
                                            Expr::val("household_id"),
                                        ]),
                                    )
                                    .eq(app
                                        .fixture
                                        .household
                                        .id
                                        .to_string()),
                                )
                                .to_owned(),
                        ),
                        "jobs",
                    )
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from("versions")
                                .and_where(
                                    Expr::col("household_id")
                                        .eq(Expr::val(app.fixture.household.id))
                                        .and(
                                            Expr::col("item_type")
                                                .eq(Expr::val("HouseholdInvitation")),
                                        ),
                                )
                                .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                                .to_owned(),
                        ),
                        "versions",
                    )
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from("security_audit_events")
                                .and_where(
                                    Expr::col("household_id")
                                        .eq(Expr::val(app.fixture.household.id)),
                                )
                                .to_owned(),
                        ),
                        "audits",
                    )
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from("household_invitation_grants")
                                .and_where(
                                    Expr::col("household_id")
                                        .eq(Expr::val(app.fixture.household.id)),
                                )
                                .to_owned(),
                        ),
                        "grants",
                    )
                    .to_owned(),
            ),
        )
        .await?
        .ok_or_else(|| DbErr::Custom("missing manager-effect state".into()))?;
    ["invitations", "jobs", "versions", "audits", "grants"]
        .iter()
        .map(|field| row.try_get::<i64>("", field))
        .collect()
}

async fn acceptance_effect_counts(
    app: &Application,
    acceptance: &super::Acceptance,
) -> Result<Vec<i64>, DbErr> {
    let memberships = Query::select()
        .expr(Expr::col(Asterisk).count())
        .from("household_memberships")
        .and_where(Expr::col("household_id").eq(acceptance.target_household.id))
        .and_where(Expr::col("person_id").eq(app.fixture.adult.id))
        .to_owned();
    let version_condition = Expr::col(("v", "item_type"))
        .eq("HouseholdInvitation")
        .and(Expr::col(("v", "item_id")).eq(acceptance.invitation.id))
        .and(Expr::col(("v", "event")).eq("update"))
        .and(Expr::expr(Func::cust("jsonb_exists").args([
            Expr::col(("v", "object_changes")).cast_as(Alias::new("jsonb")),
            Expr::val("accepted_at"),
        ])));
    let versions = Query::select()
        .expr(Expr::col(Asterisk).count())
        .from_as("versions", "v")
        .and_where(version_condition.clone())
        .and_where(
            Expr::expr(Func::cust("jsonb_extract_path_text").args([
                Expr::col(("v", "audit_context")),
                Expr::val("actor_account_id"),
            ]))
            .eq(app.fixture.account.id.to_string()),
        )
        .and_where(Expr::col(("v", "request_id")).is_not_null())
        .to_owned();
    let audits = Query::select()
        .expr(Expr::col(Asterisk).count())
        .from_as("security_audit_events", "a")
        .join_as(
            sea_orm::sea_query::JoinType::InnerJoin,
            "versions",
            "v",
            Expr::col(("a", "request_id")).eq(Expr::col(("v", "request_id"))),
        )
        .and_where(version_condition)
        .and_where(Expr::col(("a", "household_id")).eq(acceptance.target_household.id))
        .and_where(Expr::col(("a", "actor_account_id")).eq(acceptance.inviter_account.id))
        .and_where(Expr::col(("a", "actor_membership_id")).eq(acceptance.inviter_membership.id))
        .and_where(Expr::col(("a", "event_type")).eq("household_access.membership_created"))
        .and_where(
            Expr::expr(
                Func::cust("jsonb_extract_path_text")
                    .args([Expr::col(("a", "metadata")), Expr::val("target_account_id")]),
            )
            .eq(app.fixture.account.id.to_string()),
        )
        .to_owned();
    let jobs = Query::select()
        .expr(Expr::col(Asterisk).count())
        .from("pg_loco_queue")
        .and_where(
            Expr::expr(
                Func::cust("jsonb_extract_path_text")
                    .args([Expr::col("task_data"), Expr::val("household_id")]),
            )
            .is_in([
                app.fixture.household.id.to_string(),
                acceptance.target_household.id.to_string(),
            ]),
        )
        .to_owned();
    let mut counts = Vec::new();
    for query in [memberships, versions, audits, jobs] {
        let row = app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await?
            .ok_or_else(|| DbErr::Custom("missing acceptance-effect state".into()))?;
        counts.push(row.try_get_by_index::<i64>(0)?);
    }
    Ok(counts)
}

async fn signup_effect_counts(
    app: &Application,
    email: &str,
    signup: &household_invitations::Model,
) -> Result<Vec<i64>, DbErr> {
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from("accounts")
                                .and_where(Expr::col("email").eq(Expr::val(email)))
                                .to_owned(),
                        ),
                        "accounts",
                    )
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from_as("people", "p")
                                .join_as(
                                    sea_orm::sea_query::JoinType::InnerJoin,
                                    "accounts",
                                    "a",
                                    Expr::col(("a", "person_id")).eq(Expr::col(("p", "id"))),
                                )
                                .and_where(Expr::col(("a", "email")).eq(Expr::val(email)))
                                .to_owned(),
                        ),
                        "people",
                    )
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from_as("household_memberships", "m")
                                .join_as(
                                    sea_orm::sea_query::JoinType::InnerJoin,
                                    "people",
                                    "p",
                                    Expr::col(("p", "id")).eq(Expr::col(("m", "person_id"))),
                                )
                                .join_as(
                                    sea_orm::sea_query::JoinType::InnerJoin,
                                    "accounts",
                                    "a",
                                    Expr::col(("a", "person_id")).eq(Expr::col(("p", "id"))),
                                )
                                .and_where(Expr::col(("a", "email")).eq(Expr::val(email)))
                                .to_owned(),
                        ),
                        "memberships",
                    )
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from_as("pg_loco_queue", "q")
                                .and_where(
                                    Expr::expr(
                                        Func::cust("jsonb_extract_path_text")
                                            .args([Expr::col(("q", "task_data")), Expr::val("to")]),
                                    )
                                    .eq(Expr::val(email)),
                                )
                                .to_owned(),
                        ),
                        "jobs",
                    )
                    .expr_as(
                        Expr::from(
                            Query::select()
                                .expr(Expr::col(Asterisk).count())
                                .from("household_invitations")
                                .and_where(
                                    Expr::col("id")
                                        .eq(Expr::val(signup.id))
                                        .and(Expr::col("accepted_at").is_not_null()),
                                )
                                .to_owned(),
                        ),
                        "accepted",
                    )
                    .to_owned(),
            ),
        )
        .await?
        .ok_or_else(|| DbErr::Custom("missing invited-signup state".into()))?;
    ["accounts", "people", "memberships", "jobs", "accepted"]
        .iter()
        .map(|field| row.try_get::<i64>("", field))
        .collect()
}

#[tokio::test]
async fn invitation_signup_invalid_draft_rerenders_localized_form_without_effects()
-> Result<(), Box<dyn Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let acceptance=super::seed_acceptance(&app).await;
        let (signup,signup_token,signup_email)=signup_invitation(&app,&acceptance,"administrator").await?;
        let (cookie, preview) = super::super::platform::form_page(
            &app,
            "",
            &format!("/invitations/accept?token={generated_signup_token}&lang=es",generated_signup_token=signup_token),
        )
        .await;
        assert!(
            preview.contains("name=\"lang\" value=\"es\""),
            "invited signup form must carry the explicit language"
        );
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        let before = signup_effect_counts(&app,&signup_email,&signup).await?;
        assert_eq!(before, vec![0, 0, 0, 0, 0]);
        let form = [
            ("authenticity_token", csrf.as_str()),
            ("credential", "password"),
            ("invitation_token", signup_token.as_str()),
            ("lang", "es"),
            ("name", ""),
            ("date_of_birth", "1992-05-18"),
            ("email", signup_email.as_str()),
            ("password", "short"),
            ("password-confirm", "short"),
        ];
        let invalid = super::super::platform::post_form(&app, &cookie, "/create-account", &form).await;
        assert_eq!(invalid.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            invalid
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        let invalid = invalid.text().await?;
        assert!(invalid.contains("<html lang=\"es\""), "locale: {invalid}");
        assert!(
            invalid.contains(acceptance.target_household.name.as_str()),
            "invitation context must render: {invalid}"
        );
        let document = scraper::Html::parse_document(&invalid);
        for (selector, reason) in [
            ("#name[aria-invalid=\"true\"]", "invalid name flag"),
            ("#name-error", "name error description"),
            (&format!("form[action=\"/create-account\"] input[name=\"invitation_token\"][value=\"{generated_signup_token}\"]",generated_signup_token=signup_token), "invitation token draft"),
            ("form[action=\"/create-account\"] input[name=\"lang\"][value=\"es\"]", "language draft"),
            ("form[action=\"/create-account\"] input[name=\"authenticity_token\"]", "fresh csrf token"),
        ] {
            let selector = scraper::Selector::parse(selector)
                .map_err(|_| io::Error::other("invalid invalid-signup selector"))?;
            assert!(
                document.select(&selector).next().is_some(),
                "missing {reason}"
            );
        }
        assert!(invalid.contains("must be present"), "field errors shown");
        assert!(invalid.contains("value=\"1992-05-18\""), "draft date preserved");
        assert!(
            !invalid.contains("value=\"short\""),
            "password is never preserved"
        );
        assert_eq!(
            signup_effect_counts(&app,&signup_email,&signup).await?,
            before,
            "invalid invited signup must not create account, person, membership, outbox or acceptance effects"
        );
        let mut corrected = form;
        corrected[4].1 = "Synthetic invited signup";
        corrected[7].1 = "Synthetic-password-12!";
        corrected[8].1 = "Synthetic-password-12!";
        let created =
            super::super::platform::post_form(&app, &cookie, "/create-account", &corrected).await;
        assert_eq!(created.status(), reqwest::StatusCode::OK);
        assert_eq!(
            signup_effect_counts(&app,&signup_email,&signup).await?,
            vec![1, 1, 1, 1, 1],
            "corrected retry provisions the invited account once"
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_signup_dead_or_unsupported_link_recovers_in_the_selected_locale()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        let acceptance = super::seed_acceptance(&app).await;
        let (signup, signup_token, signup_email) =
            signup_invitation(&app, &acceptance, "member").await?;
        let (cookie, preview) = super::super::platform::form_page(
            &app,
            "",
            &format!(
                "/invitations/accept?token={generated_signup_token}&lang=es",
                generated_signup_token = signup_token
            ),
        )
        .await;
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        let mut form = vec![
            ("authenticity_token", csrf.clone()),
            ("credential", "password".to_owned()),
            ("invitation_token", signup_token.as_str().to_owned()),
            ("lang", "es".to_owned()),
            ("name", "Synthetic invited signup".to_owned()),
            ("date_of_birth", "1992-05-18".to_owned()),
            ("email", signup_email.as_str().to_owned()),
            ("password", "short".to_owned()),
            ("password-confirm", "short".to_owned()),
        ];
        form[3].1 = "xx".to_owned();
        let public = app.context.config.server.full_url();
        let public_url = url::Url::parse(&public).unwrap();
        let authority = match public_url.port() {
            Some(port) => format!("{}:{port}", public_url.host_str().unwrap()),
            None => public_url.host_str().unwrap().to_owned(),
        };
        let unsupported = app
            .client
            .post(format!("{}/create-account", app.origin))
            .header("cookie", &cookie)
            .header("host", authority)
            .header("origin", public.trim_end_matches('/'))
            .header("accept-language", "es")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(
                serde_urlencoded::to_string(
                    form.iter()
                        .map(|(k, v)| (*k, v.as_str()))
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            )
            .send()
            .await?;
        assert_eq!(
            unsupported.status(),
            reqwest::StatusCode::UNPROCESSABLE_ENTITY
        );
        let unsupported = unsupported.text().await?;
        assert!(
            unsupported.contains("<html lang=\"es\""),
            "fallback Accept-Language locale: {unsupported}"
        );
        assert!(unsupported.contains("Enlace de invitación no disponible"));
        form[3].1 = "es".to_owned();
        household_invitations::Entity::update_many()
            .col_expr(
                household_invitations::Column::ExpiresAt,
                Expr::val(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1)),
            )
            .filter(household_invitations::Column::Id.eq(signup.id))
            .exec(&app.fixture.admin)
            .await?;
        let expired = super::super::platform::post_form(
            &app,
            &cookie,
            "/create-account",
            &form
                .iter()
                .map(|(k, v)| (*k, v.as_str()))
                .collect::<Vec<_>>(),
        )
        .await;
        assert_eq!(expired.status(), reqwest::StatusCode::NOT_FOUND);
        let expired = expired.text().await?;
        assert!(expired.contains("<html lang=\"es\""));
        assert!(expired.contains("Enlace de invitación no disponible"));
        assert_eq!(
            signup_effect_counts(&app, &signup_email, &signup).await?,
            vec![0, 0, 0, 0, 0],
            "dead or invalid links never create effects"
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_acceptance_recovery_chooser_switches_to_the_joined_household()
-> Result<(), Box<dyn Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let acceptance=super::seed_acceptance(&app).await;
        let foreign=app.fixture.foreign_household().await;
        let foreign_household=foreign.household.clone();
        foreign.close().await;
        let revoked=app.fixture.foreign_household().await;
        let revoked_household=revoked.household.clone();
        let now=chrono::Utc::now().naive_utc();
        household_memberships::ActiveModel {household_id:Set(revoked_household.id),person_id:Set(app.fixture.adult.id),
            role:Set("member".into()),status:Set("revoked".into()),revoked_at:Set(Some(now)),
            joined_at:Set(now-chrono::Duration::days(1)),created_at:Set(now),updated_at:Set(now),..Default::default()
        }.insert(&app.fixture.admin).await?;
        revoked.close().await;

        let cookie = select_first_household(&app).await?;
        let (cookie, preview) = super::super::platform::form_page(
            &app,
            &cookie,
            &format!("/invitations/accept?token={generated_acceptance_token}&lang=es",generated_acceptance_token=acceptance.token),
        )
        .await;
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_c4_recovery_switch() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.active_household_id={generated_72002} THEN RAISE EXCEPTION 'synthetic session selection failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_c4_recovery_switch BEFORE UPDATE OF active_household_id ON identity_sessions FOR EACH ROW EXECUTE FUNCTION reject_c4_recovery_switch()",generated_72002=acceptance.target_household.id)).await?;
        let failed = super::super::platform::post_form(
            &app,
            &cookie,
            "/invitations/accept",
            &[
                ("token", acceptance.token.as_str()),
                ("lang", "es"),
                ("authenticity_token", &csrf),
            ],
        )
        .await;
        assert!(
            !failed.status().is_success(),
            "selection must fail after join"
        );
        let recovery = failed.text().await?;
        assert!(
            recovery.contains("Te has unido al hogar"),
            "honest translated recovery"
        );
        assert!(
            recovery.contains("action=\"/invitations/accept\""),
            "retry form remains available"
        );
        let committed = acceptance_effect_counts(&app,&acceptance).await?;
        assert_eq!(committed.first(), Some(&1), "committed membership");
        let document = scraper::Html::parse_document(&recovery);
        let chooser_link = scraper::Selector::parse("a[href^=\"/households/choose\"]")
            .map_err(|_| io::Error::other("invalid chooser link selector"))?;
        let chooser_href = document
            .select(&chooser_link)
            .next()
            .and_then(|link| link.value().attr("href"))
            .ok_or_else(|| io::Error::other("recovery chooser link missing"))?
            .to_owned();
        assert_eq!(
            chooser_href, "/households/choose?switch",
            "recovery must link to the explicit chooser mode"
        );
        let bounced = app
            .client
            .get(format!("{}/households/choose", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(
            bounced.url().path(),
            &format!("/households/{generated_slug}/dashboard",generated_slug=app.fixture.household.slug),
            "plain chooser still forwards to the active household dashboard"
        );
        let (cookie, chooser) =
            super::super::platform::form_page(&app, &cookie, &chooser_href).await;
        assert!(
            chooser.contains(acceptance.target_household.name.as_str())
                && chooser.contains("action=\"/households/select\""),
            "chooser lists the newly joined household: {chooser}"
        );
        let chooser_csrf = super::super::platform::hidden_field(&chooser, "authenticity_token");
        for denied_id in [foreign_household.id.to_string(), revoked_household.id.to_string(), i64::MAX.to_string()] {
            let denied = super::super::platform::post_form(
                &app,
                &cookie,
                "/households/select",
                &[
                    ("household_id", denied_id.as_str()),
                    ("authenticity_token", &chooser_csrf),
                ],
            )
            .await;
            assert!(
                denied.status().is_client_error(),
                "foreign or revoked household {denied_id}: {}",
                denied.status()
            );
        }
        let still_old = app
            .client
            .get(format!("{}/", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(
            still_old.url().path(),
            &format!("/households/{generated_slug}/dashboard",generated_slug=app.fixture.household.slug),
            "denied selections never move the active household"
        );
        app.fixture.admin.execute_unprepared("DROP TRIGGER reject_c4_recovery_switch ON identity_sessions; DROP FUNCTION reject_c4_recovery_switch()").await?;
        let selected = super::super::platform::post_form(
            &app,
            &cookie,
            "/households/select",
            &[("household_id", acceptance.target_household.id.to_string().as_str()), ("authenticity_token", &chooser_csrf)],
        )
        .await;
        assert_eq!(selected.status(), reqwest::StatusCode::OK);
        assert_eq!(
            selected.url().path(),
            &format!("/households/{generated_target_slug}/dashboard",generated_target_slug=acceptance.target_slug)
        );
        let refreshed = app
            .client
            .get(format!("{}/", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(
            refreshed.url().path(),
            &format!("/households/{generated_target_slug}/dashboard",generated_target_slug=acceptance.target_slug),
            "refresh follows the newly selected household"
        );
        assert_eq!(
            acceptance_effect_counts(&app,&acceptance).await?,
            committed,
            "chooser fallback must not repeat acceptance effects"
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    let cleanup=app.fixture.admin.execute_unprepared("DROP TRIGGER IF EXISTS reject_c4_recovery_switch ON public.identity_sessions; DROP FUNCTION IF EXISTS public.reject_c4_recovery_switch()").await;
    app.close().await;
    cleanup?;
    result
}

#[tokio::test]
async fn invitation_acceptance_selection_failure_preserves_join_and_recovers()
-> Result<(), Box<dyn Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let result = async {
        let acceptance=super::seed_acceptance(&app).await;
        let cookie = select_first_household(&app).await?;
        let (cookie, preview) = super::super::platform::form_page(&app, &cookie, &format!("/invitations/accept?token={generated_acceptance_token}&lang=es",generated_acceptance_token=acceptance.token)).await;
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_c4_selection() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.active_household_id={generated_72002} THEN RAISE EXCEPTION 'synthetic session selection failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_c4_selection BEFORE UPDATE OF active_household_id ON identity_sessions FOR EACH ROW EXECUTE FUNCTION reject_c4_selection()",generated_72002=acceptance.target_household.id)).await?;
        let failed = super::super::platform::post_form(&app, &cookie, "/invitations/accept", &[("token", acceptance.token.as_str()),("lang", "es"),("authenticity_token", &csrf)]).await;
        assert!(!failed.status().is_success(), "selection must fail after join");
        let recovery = failed.text().await?;
        assert!(recovery.contains("Te has unido al hogar"), "honest translated recovery");
        assert!(recovery.contains(&format!("name=\"token\" value=\"{generated_acceptance_token}\"",generated_acceptance_token=acceptance.token)));
        let committed = acceptance_effect_counts(&app,&acceptance).await?;
        assert_eq!(committed.len(), 4);
        assert_eq!(committed.first(), Some(&1), "committed membership");
        assert_eq!(committed.get(1), Some(&1), "attributed invitation update");
        assert_eq!(committed.get(2), Some(&1), "matching membership-created security audit");
        app.fixture.admin.execute_unprepared("DROP TRIGGER reject_c4_selection ON identity_sessions; DROP FUNCTION reject_c4_selection()").await?;
        let retry = super::super::platform::post_form(&app, &cookie, "/invitations/accept", &[("token", acceptance.token.as_str()),("lang", "es"),("authenticity_token", &csrf)]).await;
        assert_eq!(retry.status(), reqwest::StatusCode::OK);
        assert_eq!(retry.url().path(), &format!("/households/{generated_target_slug}/dashboard",generated_target_slug=acceptance.target_slug));
        assert_eq!(acceptance_effect_counts(&app,&acceptance).await?, committed, "selection retry does not repeat acceptance effects");
        app.fixture.admin.execute_raw(app.fixture.admin.get_database_backend().build(&Query::update().table("household_memberships").value("status",Expr::val("revoked")).value("revoked_at",Expr::expr(Func::cust("now"))).and_where(Expr::col("household_id").eq(Expr::val(acceptance.target_household.id)).and(Expr::col("person_id").eq(Expr::val(app.fixture.adult.id)))).to_owned())).await?;
        let denied = super::super::platform::post_form(&app, &cookie, "/invitations/accept", &[("token", acceptance.token.as_str()),("lang", "es"),("authenticity_token", &csrf)]).await;
        assert_eq!(denied.status(), reqwest::StatusCode::NOT_FOUND);
        let denied = denied.text().await?;
        assert!(denied.contains("Enlace de invitación no disponible"));
        assert_eq!(acceptance_effect_counts(&app,&acceptance).await?, committed, "revoked replay must not restore membership or duplicate effects");
        let revoked = app.fixture.admin.query_one_raw(app.fixture.admin.get_database_backend().build(&Query::select().expr(Expr::col("status")).expr_as(Expr::col("revoked_at").is_not_null(),"revoked").from("household_memberships").and_where(Expr::col("household_id").eq(Expr::val(acceptance.target_household.id)).and(Expr::col("person_id").eq(Expr::val(app.fixture.adult.id)))).to_owned())).await?.ok_or_else(|| io::Error::other("missing revoked membership"))?;
        assert_eq!(revoked.try_get::<String>("", "status")?, "revoked");
        assert!(revoked.try_get::<bool>("", "revoked")?);
        Ok::<_, Box<dyn Error>>(())
    }.await;
    let cleanup=app.fixture.admin.execute_unprepared("DROP TRIGGER IF EXISTS reject_c4_selection ON public.identity_sessions; DROP FUNCTION IF EXISTS public.reject_c4_selection()").await;
    app.close().await;
    cleanup?;
    result
}

fn page_email(app: &Application, index: i32) -> String {
    format!(
        "browser-page-{index}-{}@example.test",
        app.fixture.client_uuid
    )
}
fn browser_email(app: &Application, label: &str) -> String {
    format!(
        "{}-{}@example.test",
        label.trim_end_matches("@example.test"),
        app.fixture.client_uuid
    )
}
async fn local_invitation(
    app: &Application,
    label: &str,
    role: &str,
    days: i64,
    age: i64,
    status: &str,
) -> Result<household_invitations::Model, DbErr> {
    let now = chrono::Utc::now().naive_utc();
    household_invitations::ActiveModel {
        household_id: Set(app.fixture.household.id),
        email: Set(browser_email(app, label)),
        membership_role: Set(role.into()),
        token_digest: Set(uuid::Uuid::new_v4().to_string()),
        invited_by_membership_id: Set(app.fixture.membership.id),
        expires_at: Set(now + chrono::Duration::days(days)),
        accepted_at: Set((status == "accepted").then_some(now)),
        revoked_at: Set((status == "revoked").then_some(now)),
        created_at: Set(now - chrono::Duration::days(age)),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
}
async fn signup_invitation(
    app: &Application,
    acceptance: &super::Acceptance,
    role: &str,
) -> Result<(household_invitations::Model, String, String), DbErr> {
    let token = uuid::Uuid::new_v4().to_string();
    let email = browser_email(app, "invited-signup@example.test");
    let now = chrono::Utc::now().naive_utc();
    let invitation = household_invitations::ActiveModel {
        household_id: Set(acceptance.target_household.id),
        email: Set(email.clone()),
        membership_role: Set(role.into()),
        token_digest: Set(hex::encode(Sha256::digest(token.as_bytes()))),
        invited_by_membership_id: Set(acceptance.inviter_membership.id),
        expires_at: Set(now + chrono::Duration::days(7)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await?;
    Ok((invitation, token, email))
}
