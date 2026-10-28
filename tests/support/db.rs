use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

/// A private database cloned from a prepared template with its own restricted
/// runtime role. Lifecycle statements live here only; fixture data belongs in
/// `support::fixtures`.
pub struct OwnedDatabase {
    pub admin: DatabaseConnection,
    pub runtime: DatabaseConnection,
    pub runtime_uri: String,
    control_uri: String,
    name: String,
    role: String,
    marker: String,
}

impl OwnedDatabase {
    pub async fn clone(
        control_uri: &str,
        template: &str,
        db_prefix: &'static str,
        role_prefix: &'static str,
    ) -> Self {
        validate_control_uri(control_uri);
        for identifier in [template, db_prefix, role_prefix] {
            assert!(
                !identifier.is_empty()
                    && identifier.bytes().all(|byte| byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || byte == b'_')
            );
        }
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("{db_prefix}{suffix}");
        let role = format!("{role_prefix}{suffix}");
        assert!(name.len() <= 63 && role.len() <= 63);
        let marker = format!("medtracker-browser:{suffix}");
        let control = Database::connect(control_uri).await.unwrap();
        // SeaORM cannot administer PostgreSQL databases or login roles. Only
        // validated identifiers and UUID ownership markers enter these statements.
        control
            .execute_unprepared(&format!("CREATE DATABASE {name} TEMPLATE {template}"))
            .await
            .unwrap();
        control
            .execute_unprepared(&format!("COMMENT ON DATABASE {name} IS '{marker}'"))
            .await
            .unwrap();
        control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app")).await.unwrap();
        control
            .execute_unprepared(&format!("COMMENT ON ROLE {role} IS '{marker}'"))
            .await
            .unwrap();
        control.close().await.unwrap();
        let mut admin_uri = url::Url::parse(control_uri).unwrap();
        admin_uri.set_path(&name);
        let admin = Database::connect(admin_uri.to_string()).await.unwrap();
        let mut runtime_uri = admin_uri;
        runtime_uri
            .set_username(&role)
            .map_err(|()| "set runtime username")
            .unwrap();
        runtime_uri
            .set_password(Some("password"))
            .map_err(|()| "set runtime password")
            .unwrap();
        let runtime_uri = runtime_uri.to_string();
        let runtime = Database::connect(&runtime_uri).await.unwrap();
        Self {
            admin,
            runtime,
            runtime_uri,
            control_uri: control_uri.to_string(),
            name,
            role,
            marker,
        }
    }

    pub async fn close(self) {
        validate_control_uri(&self.control_uri);
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
        let control = Database::connect(&self.control_uri).await.unwrap();
        for (catalog, column, object, name) in [
            ("pg_database", "datname", "pg_database", &self.name),
            ("pg_roles", "rolname", "pg_authid", &self.role),
        ] {
            let query = Query::select()
                .expr_as(
                    Func::cust("shobj_description").args([Expr::col("oid"), Expr::val(object)]),
                    "owner",
                )
                .from(("pg_catalog", catalog))
                .and_where(Expr::col(column).eq(name.as_str()))
                .to_owned();
            let row = control
                .query_one_raw(control.get_database_backend().build(&query))
                .await
                .unwrap()
                .expect("owned fixture resource must exist");
            assert_eq!(
                row.try_get::<Option<String>>("", "owner")
                    .unwrap()
                    .as_deref(),
                Some(self.marker.as_str()),
                "refusing to remove an unowned fixture resource"
            );
        }
        control
            .execute_unprepared(&format!("DROP DATABASE {} WITH (FORCE)", self.name))
            .await
            .unwrap();
        control
            .execute_unprepared(&format!("DROP ROLE {}", self.role))
            .await
            .unwrap();
        control.close().await.unwrap();
    }
}

fn validate_control_uri(uri: &str) {
    assert!(
        crate::template::is_owned_database_endpoint(&url::Url::parse(uri).unwrap()),
        "unowned control endpoint"
    );
    crate::template::require_owned_database();
}

#[tokio::test]
#[should_panic(expected = "unowned control endpoint")]
async fn rejected_control_endpoint_cannot_reach_cleanup() {
    OwnedDatabase {
        admin: DatabaseConnection::default(),
        runtime: DatabaseConnection::default(),
        runtime_uri: String::new(),
        control_uri: "postgres://medtracker:medtracker_password@production/medtracker_loco".into(),
        name: "browser_invalid".into(),
        role: "browser_runtime_invalid".into(),
        marker: String::new(),
    }
    .close()
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn owned_clone_cleanup_checks_both_markers_before_removing_either_resource() {
    use futures_util::FutureExt;
    use std::panic::AssertUnwindSafe;

    let uri = crate::template::owned_database_url();
    crate::template::prepare(&uri, "owned_lifecycle_template", |_| async {}).await;
    let owned = OwnedDatabase::clone(
        &uri,
        "owned_lifecycle_template",
        "browser_",
        "browser_runtime_",
    )
    .await;
    let name = owned.name.clone();
    let role = owned.role.clone();
    let marker = owned.marker.clone();
    let runtime_uri = owned.runtime_uri.clone();
    let control = Database::connect(&uri).await.unwrap();
    control
        .execute_unprepared(&format!("COMMENT ON ROLE {role} IS 'unowned'"))
        .await
        .unwrap();
    let failure = AssertUnwindSafe(owned.close())
        .catch_unwind()
        .await
        .unwrap_err();
    let message = failure.downcast_ref::<String>().unwrap();
    assert!(message.contains("refusing to remove an unowned fixture resource"));
    for (catalog, column, identifier) in [
        ("pg_database", "datname", &name),
        ("pg_roles", "rolname", &role),
    ] {
        let query = Query::select()
            .column(column)
            .from(("pg_catalog", catalog))
            .and_where(Expr::col(column).eq(identifier.as_str()))
            .to_owned();
        assert!(
            control
                .query_one_raw(control.get_database_backend().build(&query))
                .await
                .unwrap()
                .is_some()
        );
    }
    control
        .execute_unprepared(&format!("COMMENT ON ROLE {role} IS '{marker}'"))
        .await
        .unwrap();
    let mut admin_uri = url::Url::parse(&uri).unwrap();
    admin_uri.set_path(&name);
    OwnedDatabase {
        admin: Database::connect(admin_uri.as_str()).await.unwrap(),
        runtime: Database::connect(&runtime_uri).await.unwrap(),
        runtime_uri,
        control_uri: uri,
        name: name.clone(),
        role: role.clone(),
        marker,
    }
    .close()
    .await;
    for (catalog, column, identifier) in [
        ("pg_database", "datname", name),
        ("pg_roles", "rolname", role),
    ] {
        let query = Query::select()
            .column(column)
            .from(("pg_catalog", catalog))
            .and_where(Expr::col(column).eq(identifier))
            .to_owned();
        assert!(
            control
                .query_one_raw(control.get_database_backend().build(&query))
                .await
                .unwrap()
                .is_none()
        );
    }
    control.close().await.unwrap();
}
