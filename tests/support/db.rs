use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

/// A private database cloned from a prepared template with its own restricted
/// runtime role. Lifecycle statements live here only; fixture data belongs in
/// `support::fixtures`.
pub struct OwnedDatabase {
    pub admin: DatabaseConnection,
    pub runtime: DatabaseConnection,
    pub runtime_uri: String,
    control_uri: String,
    db_prefix: &'static str,
    role_prefix: &'static str,
}

impl OwnedDatabase {
    pub async fn clone(
        control_uri: &str,
        template: &str,
        db_prefix: &'static str,
        role_prefix: &'static str,
    ) -> Self {
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("{db_prefix}{suffix}");
        let role = format!("{role_prefix}{suffix}");
        let control = Database::connect(control_uri).await.unwrap();
        control
            .execute_unprepared(&format!("CREATE DATABASE {name} TEMPLATE {template}"))
            .await
            .unwrap();
        control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app")).await.unwrap();
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
            db_prefix,
            role_prefix,
        }
    }

    pub async fn close(self) {
        let uri = url::Url::parse(&self.runtime_uri).unwrap();
        let name = uri.path().trim_start_matches('/');
        let role = uri.username();
        assert!(
            name.strip_prefix(self.db_prefix)
                .is_some_and(|suffix| uuid::Uuid::parse_str(suffix).is_ok())
        );
        assert!(
            role.strip_prefix(self.role_prefix)
                .is_some_and(|suffix| uuid::Uuid::parse_str(suffix).is_ok())
        );
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
        let control = Database::connect(&self.control_uri).await.unwrap();
        control
            .execute_unprepared(&format!("DROP DATABASE {name} WITH (FORCE)"))
            .await
            .unwrap();
        control
            .execute_unprepared(&format!("DROP ROLE {role}"))
            .await
            .unwrap();
        control.close().await.unwrap();
    }
}
