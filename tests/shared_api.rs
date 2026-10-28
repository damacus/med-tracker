#[path = "support/authentication_fixture.rs"]
mod authentication_fixture;
#[path = "support/scenario.rs"]
mod scenario;
#[path = "support/session_fixture.rs"]
mod session_fixture;

use med_tracker::models::{_entities::household_memberships, access};
use sea_orm::EntityTrait;

#[tokio::test]
async fn concurrent_households_share_one_database_without_rls_leakage() {
    let first = scenario::Scenario::new().await;
    let second = scenario::Scenario::new().await;
    assert_eq!(first.database_name().await, second.database_name().await);
    assert_ne!(first.account.id, second.account.id);
    assert_ne!(first.account.email, second.account.email);
    assert_ne!(first.household.id, second.household.id);
    let first_scope = first.scope();
    let second_scope = second.scope();
    let (first_tenant, second_tenant) = tokio::join!(
        access::begin(&first.runtime, &first_scope, None),
        access::begin(&second.runtime, &second_scope, None),
    );
    let first_tenant = first_tenant.unwrap();
    let second_tenant = second_tenant.unwrap();
    assert_eq!(
        household_memberships::Entity::find()
            .all(first_tenant.transaction())
            .await
            .unwrap()
            .into_iter()
            .map(|membership| membership.id)
            .collect::<Vec<_>>(),
        vec![first.membership.id],
    );
    assert_eq!(
        household_memberships::Entity::find()
            .all(second_tenant.transaction())
            .await
            .unwrap()
            .into_iter()
            .map(|membership| membership.id)
            .collect::<Vec<_>>(),
        vec![second.membership.id],
    );
    first_tenant.commit().await.unwrap();
    second_tenant.commit().await.unwrap();
    let mut foreign_scope = first.scope();
    foreign_scope.household_id = second.household.id;
    assert!(
        access::begin(&first.runtime, &foreign_scope, None)
            .await
            .is_err()
    );
    first.close().await;
    second.close().await;
}

#[tokio::test]
async fn standard_loco_requests_use_the_shared_database_and_real_identity() {
    let fixture = scenario::Scenario::new().await;
    loco_rs::testing::request::request::<med_tracker::app::App, _, _>(
        |request, context| async move {
            let service = context
                .shared_store
                .get::<med_tracker::models::identity::better_auth::IdentityService>()
                .unwrap();
            let mut sign_in = better_auth_core::AuthRequest::new(
                better_auth_core::HttpMethod::Post,
                "/sign-in/email",
            );
            sign_in
                .headers
                .insert("origin".into(), context.config.server.full_url());
            sign_in
                .headers
                .insert("content-type".into(), "application/json".into());
            sign_in.body = Some(
                serde_json::to_vec(
                    &serde_json::json!({"email":fixture.account.email,"password":"password"}),
                )
                .unwrap(),
            );
            let response = med_tracker::models::identity::better_auth::dispatch(
                &service,
                sign_in,
                "shared-loco-request".into(),
            )
            .await
            .unwrap();
            assert_eq!(response.status, 200);
            let cookie = response
                .headers
                .get("set-cookie")
                .unwrap()
                .split(';')
                .next()
                .unwrap();
            let session = request
                .get("/api/auth/get-session")
                .add_header("cookie", cookie)
                .await;
            session.assert_status_ok();
            assert!(session.text().contains(&fixture.account.email));
            let dashboard = request
                .get(&format!("/households/{}/dashboard", fixture.household.slug))
                .add_header("cookie", cookie)
                .await;
            dashboard.assert_status_ok();
            assert!(dashboard.text().contains("id=\"dashboard\""));
            assert!(dashboard.text().contains(&fixture.household.slug));
            if let Some(queue) = &context.queue_provider {
                queue.shutdown().unwrap();
            }
            context.db.close().await.unwrap();
            drop(request);
            fixture.close().await;
        },
    )
    .await;
}

async fn persisted_identity_quota(
    connection: &sea_orm::DatabaseConnection,
) -> Vec<(String, i32, chrono::DateTime<chrono::Utc>)> {
    use sea_orm::{ConnectionTrait, sea_query::Query};
    let query = Query::select()
        .columns(["bucket", "attempts", "expires_at"])
        .from("identity_rate_limits")
        .order_by("bucket", sea_orm::sea_query::Order::Asc)
        .to_owned();
    connection
        .query_all_raw(connection.get_database_backend().build(&query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| {
            (
                row.try_get("", "bucket").unwrap(),
                row.try_get("", "attempts").unwrap(),
                row.try_get("", "expires_at").unwrap(),
            )
        })
        .collect()
}

#[tokio::test]
async fn native_browser_session_fixture_uses_real_routes_without_consuming_source_quota() {
    use med_tracker::models::entities::identity_session;
    use sea_orm::{ColumnTrait, QueryFilter};

    let own = scenario::Scenario::new().await;
    let foreign = scenario::Scenario::new().await;
    let boot = loco_rs::testing::request::boot_test::<med_tracker::app::App>()
        .await
        .unwrap();
    let context = &boot.app_context;
    assert!(!context.config.database.dangerously_truncate);
    assert!(context.config.database.uri == std::env::var("DATABASE_URL").unwrap());
    assert!(
        household_memberships::Entity::find()
            .all(&context.db)
            .await
            .unwrap()
            .is_empty(),
        "unscoped runtime credentials must retain RLS"
    );
    let request = loco_rs::TestServer::new(
        boot.router
            .clone()
            .unwrap()
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .unwrap();
    let quota_before = persisted_identity_quota(&own.admin).await;
    let cookie = session_fixture::browser_cookie(context, own.account.id).await;
    assert!(!cookie.is_empty());
    let service = context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let mut native_request =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Get, "/get-session");
    native_request
        .headers
        .insert("cookie".into(), cookie.clone());
    assert!(
        service
            .context()
            .session_manager()
            .extract_session_token(&native_request)
            .is_some(),
        "Better Auth must recognise its issued browser cookie"
    );
    let stored = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(own.account.id))
        .all(&own.admin)
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert!(stored[0].active);
    assert_eq!(persisted_identity_quota(&own.admin).await, quota_before);

    let session = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    session.assert_status_ok();
    let body: serde_json::Value = session.json();
    assert_eq!(body["user"]["id"], own.account.id.to_string());
    assert_eq!(body["user"]["email"], own.account.email);
    let dashboard = request
        .get(&format!("/households/{}/dashboard", own.household.slug))
        .add_header("cookie", cookie.clone())
        .await;
    dashboard.assert_status_ok();
    assert!(dashboard.text().contains("id=\"dashboard\""));
    assert!(dashboard.text().contains(&own.household.slug));
    let denied = request
        .get(&format!("/households/{}/dashboard", foreign.household.slug))
        .add_header("cookie", cookie)
        .await;
    assert_eq!(denied.status_code(), 403);
    assert!(!denied.text().contains("id=\"dashboard\""));
    assert!(!denied.text().contains(&foreign.account.email));
    assert_eq!(persisted_identity_quota(&own.admin).await, quota_before);

    drop(request);
    if let Some(queue) = &context.queue_provider {
        queue.shutdown().unwrap();
    }
    context.db.clone().close().await.unwrap();
    own.close().await;
    foreign.close().await;
}

#[tokio::test]
async fn guarded_requests_share_the_generous_test_source_limit() {
    let fixture = scenario::Scenario::new().await;
    let boot = loco_rs::testing::request::boot_test::<med_tracker::app::App>()
        .await
        .unwrap();
    let server = loco_rs::TestServer::new(
        boot.router
            .clone()
            .unwrap()
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .unwrap();
    for attempt in 0..45 {
        let response = server
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&serde_json::json!({
                "email": format!("missing-shared-{}-{attempt}@example.test", fixture.household.slug),
                "password": "wrong-password"
            }))
            .await;
        assert_ne!(
            response.status_code(),
            429,
            "attempt {attempt}: {}",
            response.text()
        );
    }
    drop(server);
    if let Some(queue) = &boot.app_context.queue_provider {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    fixture.close().await;
}

#[tokio::test]
async fn authentication_quota_fixture_resets_only_source_buckets_between_real_journeys() {
    use med_tracker::models::_entities::identity_rate_limits;
    use sea_orm::{ActiveModelTrait, Set};

    unsafe {
        std::env::set_var("MEDTRACKER_AUTH_LIMIT_SOURCE_MAXIMUM", "40");
        std::env::set_var("MEDTRACKER_AUTH_LIMIT_SOURCE_WINDOW_SECONDS", "60");
    }

    let fixture = scenario::Scenario::new().await;
    let unrelated = format!("synthetic-unrelated-{}", uuid::Uuid::new_v4());
    let retained = identity_rate_limits::ActiveModel {
        bucket: Set(unrelated.clone()),
        attempts: Set(7),
        expires_at: Set((chrono::Utc::now() + chrono::Duration::minutes(5)).fixed_offset()),
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    authentication_fixture::reset_source_quota().await;
    let boot = loco_rs::testing::request::boot_test::<med_tracker::app::App>()
        .await
        .unwrap();
    let server = loco_rs::TestServer::new(
        boot.router
            .clone()
            .unwrap()
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .unwrap();
    for attempt in 0..41 {
        let response = server
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&serde_json::json!({
                "email": format!("missing-{}-{attempt}@example.test", fixture.household.slug),
                "password": "wrong-password"
            }))
            .await;
        assert_eq!(
            response.status_code(),
            if attempt < 40 { 401 } else { 429 },
            "{}",
            response.text()
        );
        if attempt == 40 {
            assert!(
                response
                    .header("retry-after")
                    .to_str()
                    .unwrap()
                    .parse::<u32>()
                    .unwrap()
                    > 0
            );
        }
    }
    let next_boot = loco_rs::testing::request::boot_test::<med_tracker::app::App>()
        .await
        .unwrap();
    let next_server = loco_rs::TestServer::new(
        next_boot
            .router
            .clone()
            .unwrap()
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .unwrap();
    let still_limited = next_server
        .post("/api/auth/sign-in/email")
        .add_header("origin", next_boot.app_context.config.server.full_url())
        .json(&serde_json::json!({"email":fixture.account.email,"password":"password"}))
        .await;
    assert_eq!(
        still_limited.status_code(),
        429,
        "a fresh router must preserve the database quota"
    );
    authentication_fixture::reset_source_quota().await;
    let next_journey = next_server
        .post("/api/auth/sign-in/email")
        .add_header("origin", next_boot.app_context.config.server.full_url())
        .json(&serde_json::json!({"email":fixture.account.email,"password":"password"}))
        .await;
    next_journey.assert_status_ok();
    assert_eq!(
        identity_rate_limits::Entity::find_by_id(unrelated)
            .one(&fixture.admin)
            .await
            .unwrap(),
        Some(retained)
    );
    drop(server);
    drop(next_server);
    for context in [boot.app_context, next_boot.app_context] {
        if let Some(queue) = &context.queue_provider {
            queue.shutdown().unwrap();
        }
        context.db.close().await.unwrap();
    }
    fixture.close().await;
}
