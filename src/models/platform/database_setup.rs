use base64::Engine;
use loco_rs::{
    app::AppContext,
    config::{Config, QueueConfig},
    task::{Task, TaskInfo, Vars},
};
use migration::{Migrator, MigratorTrait, provision::Access};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    sea_query::{Alias, Expr, ExprTrait, JoinType, Query},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

pub struct DatabaseSetupTask;

#[derive(Debug)]
pub(crate) struct SetupError(&'static str);

type SetupResult<T> = Result<T, SetupError>;

impl std::fmt::Display for SetupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for SetupError {}

impl From<SetupError> for loco_rs::Error {
    fn from(value: SetupError) -> Self {
        Self::string(value.0)
    }
}

#[derive(Default, Deserialize)]
struct Settings {
    runtime_url: Option<String>,
    update_url: Option<String>,
    runtime: Option<super::database_connection::Credentials>,
    update: Option<super::database_connection::Credentials>,
    output_directory: Option<PathBuf>,
    source_directory: Option<PathBuf>,
    #[serde(default)]
    persist_config: bool,
    stored_database_url: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Overlay {
    database: DatabaseAccess,
    #[serde(skip_serializing_if = "Option::is_none")]
    queue: Option<QueueAccess>,
    settings: serde_json::Value,
}

#[derive(Default, Deserialize, Serialize)]
struct DatabaseAccess {
    uri: String,
    auto_migrate: bool,
    dangerously_recreate: bool,
    dangerously_truncate: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind")]
enum QueueAccess {
    Postgres { uri: String },
}

impl Settings {
    fn from_config(config: &Config) -> SetupResult<Self> {
        config
            .settings
            .as_ref()
            .and_then(|settings| settings.get("database_setup"))
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| error("Invalid database setup settings"))
            .map(Option::unwrap_or_default)
    }
}

fn error(message: &'static str) -> SetupError {
    SetupError(message)
}

fn random<const N: usize>() -> SetupResult<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|_| error("Could not generate database credentials"))?;
    Ok(bytes)
}

fn saved_config(path: &Path) -> SetupResult<Option<Overlay>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(error("Setup configuration must be a regular private file"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o077 != 0 {
                    return Err(error(
                        "Setup configuration must have owner-only permissions",
                    ));
                }
            }
            let contents =
                std::fs::read(path).map_err(|_| error("Cannot read saved setup configuration"))?;
            serde_yaml_ng::from_slice(&contents)
                .map(Some)
                .map_err(|_| error("Invalid saved setup configuration"))
        }
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(error("Cannot inspect saved setup configuration")),
    }
}

fn credential(
    supplied: Option<&str>,
    saved: Option<&Overlay>,
    bootstrap: &Url,
    kind: &str,
) -> SetupResult<Url> {
    let existing = saved
        .map(|config| {
            serde_json::from_value::<Settings>(config.settings["database_setup"].clone())
                .map_err(|_| error("Invalid saved setup credentials"))
        })
        .transpose()?
        .and_then(|settings| settings.stored_database_url);
    let supplied = supplied.filter(|value| !value.trim().is_empty());
    if let (Some(supplied), Some(existing)) = (supplied, existing.as_deref())
        && supplied != existing
    {
        return Err(error(
            "Supplied credentials differ from saved setup; explicit credential rotation is required",
        ));
    }
    let uri = if let Some(value) = supplied.or(existing.as_deref()) {
        Url::parse(value).map_err(|_| error("Invalid application database URL"))?
    } else {
        let digest = hex::encode(Sha256::digest(bootstrap.path().as_bytes()));
        let identity = digest.chars().take(32).collect::<String>();
        let mut uri = bootstrap.clone();
        uri.set_username(&format!("mt_{identity}_{kind}"))
            .map_err(|()| error("Cannot create database login name"))?;
        uri.set_password(Some(&hex::encode(random::<32>()?)))
            .map_err(|()| error("Cannot create database password"))?;
        uri
    };
    if !matches!(uri.scheme(), "postgres" | "postgresql")
        || uri.host_str() != bootstrap.host_str()
        || uri.port().unwrap_or(5432) != bootstrap.port().unwrap_or(5432)
        || uri.path() != bootstrap.path()
        || uri.fragment().is_some()
        || uri.username().is_empty()
        || uri.username().len() > 63
        || !uri
            .username()
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || uri.password().is_none_or(str::is_empty)
    {
        return Err(error(
            "Application credentials must use a distinct login on the same PostgreSQL database",
        ));
    }
    Ok(uri)
}

fn output_config(source: &Config, uri: &Url, saved: Option<&Overlay>) -> SetupResult<Overlay> {
    let mut config = Overlay {
        database: DatabaseAccess::default(),
        queue: None,
        settings: serde_json::json!({}),
    };
    let default_uri =
        serde_json::to_string(uri.as_str()).map_err(|_| error("Cannot encode database URL"))?;
    let value = format!("<%= get_env(name=\"DATABASE_URL\", default={default_uri}) %>");
    config.database.uri = value.clone();
    config.database.auto_migrate = false;
    config.database.dangerously_recreate = false;
    config.database.dangerously_truncate = false;
    if matches!(source.queue, Some(QueueConfig::Postgres(_))) {
        config.queue = Some(QueueAccess::Postgres { uri: value });
    }
    let mut settings = serde_json::json!({});
    settings["database_setup"] = serde_json::json!({"stored_database_url": uri.as_str()});
    for (section, key, session_key) in [
        ("dose_occurrences", "signing_key", false),
        ("browser_session", "key", true),
    ] {
        let existing = source
            .settings
            .as_ref()
            .and_then(|settings| {
                settings
                    .get(section)
                    .and_then(|value| value.get(key))
                    .and_then(serde_json::Value::as_str)
            })
            .filter(|value| !value.is_empty())
            .or_else(|| {
                saved
                    .map(|config| &config.settings)
                    .and_then(|value| value.get(section))
                    .and_then(|value| value.get(key))
                    .and_then(serde_json::Value::as_str)
            });
        let value = if let Some(value) = existing {
            value.to_owned()
        } else if session_key {
            base64::engine::general_purpose::STANDARD.encode(random::<64>()?)
        } else {
            hex::encode(random::<64>()?)
        };
        let name = if session_key {
            "MEDTRACKER_SESSION_KEY"
        } else {
            "AUTH_SESSION_SECRET"
        };
        let default =
            serde_json::to_string(&value).map_err(|_| error("Cannot encode session key"))?;
        settings[section][key] = serde_json::Value::String(format!(
            "<%= get_env(name=\"{name}\", default={default}) %>"
        ));
    }
    config.settings = settings;
    Ok(config)
}

fn persist(path: &Path, config: &Overlay, source: &Path) -> SetupResult<()> {
    use std::fs::DirBuilder;
    let parent = path
        .parent()
        .ok_or_else(|| error("Invalid setup configuration directory"))?;
    let mut directory = DirBuilder::new();
    directory.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory
        .create(parent)
        .map_err(|_| error("Cannot create private setup directory; check volume ownership"))?;
    if std::fs::symlink_metadata(parent)
        .map_err(|_| error("Cannot inspect setup directory"))?
        .file_type()
        .is_symlink()
    {
        return Err(error(
            "Setup configuration directory cannot be a symbolic link",
        ));
    }
    let base_path = parent.join("production.yaml");
    if !base_path.exists() {
        let base = std::fs::read(source.join("production.yaml"))
            .map_err(|_| error("Cannot read the production configuration template"))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| error("Cannot prepare production configuration"))?;
        temporary
            .write_all(&base)
            .map_err(|_| error("Cannot copy production configuration"))?;
        temporary
            .persist_noclobber(&base_path)
            .map_err(|_| error("Cannot persist production configuration"))?;
    }
    let contents =
        serde_yaml_ng::to_string(config).map_err(|_| error("Cannot encode setup configuration"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| error("Cannot create private setup configuration"))?;
    temporary
        .write_all(contents.as_bytes())
        .map_err(|_| error("Cannot write setup configuration"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| error("Cannot persist setup configuration"))?;
    temporary.persist_noclobber(path).map_err(|_| {
        error("Setup configuration already exists or cannot be persisted; retry setup")
    })?;
    Ok(())
}

async fn check_schema(db: &DatabaseConnection) -> SetupResult<()> {
    let manager = migration::SchemaManager::new(db);
    if manager
        .has_table("seaql_migrations")
        .await
        .map_err(database_error)?
    {
        let first = Migrator::migrations()
            .into_iter()
            .next()
            .ok_or_else(|| error("Initial schema definition unavailable"))?;
        let query = Query::select()
            .column(Alias::new("version"))
            .from(Alias::new("seaql_migrations"))
            .and_where(Expr::col(Alias::new("version")).eq(first.name()))
            .to_owned();
        if db
            .query_one_raw(db.get_database_backend().build(&query))
            .await
            .map_err(database_error)?
            .is_some()
        {
            return Ok(());
        }
    }
    let query = Query::select()
        .column((Alias::new("c"), Alias::new("oid")))
        .from_as(
            (Alias::new("pg_catalog"), Alias::new("pg_class")),
            Alias::new("c"),
        )
        .join_as(
            JoinType::InnerJoin,
            (Alias::new("pg_catalog"), Alias::new("pg_namespace")),
            Alias::new("n"),
            Expr::col((Alias::new("c"), Alias::new("relnamespace")))
                .equals((Alias::new("n"), Alias::new("oid"))),
        )
        .and_where(
            Expr::col((Alias::new("n"), Alias::new("nspname"))).is_in(["public", "med_tracker"]),
        )
        .and_where(
            Expr::col((Alias::new("c"), Alias::new("relkind")))
                .is_in(["r", "p", "v", "m", "S", "f"]),
        )
        .and_where(Expr::col((Alias::new("c"), Alias::new("relname"))).ne("seaql_migrations"))
        .to_owned();
    if db
        .query_one_raw(db.get_database_backend().build(&query))
        .await
        .map_err(database_error)?
        .is_some()
    {
        return Err(error(
            "Setup requires an empty database or a recognised MedTracker installation; existing data was not changed",
        ));
    }
    Ok(())
}

fn database_error(cause: sea_orm::DbErr) -> SetupError {
    let category = match cause {
        sea_orm::DbErr::Conn(_) => "connection",
        sea_orm::DbErr::Exec(_) => "execution",
        sea_orm::DbErr::Query(_) => "query",
        _ => "database",
    };
    tracing::error!(
        category,
        "Database setup failed; database details and credentials are redacted"
    );
    error(
        "Database setup failed. Check setup privileges, existing role compatibility and the supplied credentials",
    )
}

async fn verify_login(uri: &Url, access: Access) -> SetupResult<()> {
    let mut options = ConnectOptions::new(uri.as_str());
    options
        .max_connections(1)
        .min_connections(1)
        .sqlx_logging(false)
        .connect_timeout(Duration::from_secs(5));
    let db = Database::connect(options).await.map_err(database_error)?;
    let result = migration::provision::validate(&db, uri.username(), access)
        .await
        .map_err(database_error);
    db.close().await.map_err(database_error)?;
    result
}

pub(crate) async fn validate_runtime(ctx: &AppContext) -> SetupResult<()> {
    let uri =
        Url::parse(&ctx.config.database.uri).map_err(|_| error("Invalid runtime database URL"))?;
    migration::provision::validate(&ctx.db, uri.username(), Access::Runtime).await.map_err(|_| {
        error("Production startup requires restricted application credentials; use database-setup for installation and db migrate for schema updates")
    })
}

#[async_trait::async_trait]
impl Task for DatabaseSetupTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "database-setup".into(),
            detail: "Install MedTracker and provision restricted runtime and schema-update access"
                .into(),
        }
    }

    async fn run(&self, ctx: &AppContext, _: &Vars) -> loco_rs::Result<()> {
        let settings = Settings::from_config(&ctx.config)?;
        let bootstrap = Url::parse(&ctx.config.database.uri)
            .map_err(|_| error("Invalid setup database URL"))?;
        let runtime_url = settings
            .runtime_url
            .as_deref()
            .filter(|value| !value.is_empty());
        let update_url = settings
            .update_url
            .as_deref()
            .filter(|value| !value.is_empty());
        let runtime_fields = match (runtime_url, &settings.runtime) {
            (None, Some(fields)) => fields.connection_uri(&bootstrap)?,
            _ => None,
        };
        let update_fields = match (update_url, &settings.update) {
            (None, Some(fields)) => fields.connection_uri(&bootstrap)?,
            _ => None,
        };
        let runtime_url = runtime_url.or_else(|| runtime_fields.as_ref().map(Url::as_str));
        let update_url = update_url.or_else(|| update_fields.as_ref().map(Url::as_str));
        let directory = settings
            .output_directory
            .as_deref()
            .unwrap_or_else(|| Path::new("setup"));
        let runtime_path = directory.join("runtime/production.local.yaml");
        let update_path = directory.join("update/production.local.yaml");
        let runtime_saved = saved_config(&runtime_path)?;
        let update_saved = saved_config(&update_path)?;
        let runtime = credential(runtime_url, runtime_saved.as_ref(), &bootstrap, "runtime")?;
        let update = credential(update_url, update_saved.as_ref(), &bootstrap, "update")?;
        if runtime.username() == update.username()
            || runtime.username() == bootstrap.username()
            || update.username() == bootstrap.username()
        {
            return Err(error("Setup, runtime and schema-update logins must be distinct").into());
        }
        let mut options = ConnectOptions::new(bootstrap.as_str());
        options
            .max_connections(1)
            .min_connections(1)
            .sqlx_logging(false);
        let db = Database::connect(options).await.map_err(database_error)?;
        let result: SetupResult<()> = async {
            check_schema(&db).await?;
            for (uri, access) in [(&runtime, Access::Runtime), (&update, Access::Update)] {
                migration::provision::preflight(&db, uri.username(), access).await.map_err(database_error)?;
            }
            let generated = runtime_url.is_none() || update_url.is_none();
            if generated || settings.persist_config {
                for (path, uri, saved) in [(&runtime_path, &runtime, runtime_saved.as_ref()), (&update_path, &update, update_saved.as_ref())] {
                    if saved.is_none() { persist(path, &output_config(&ctx.config, uri, saved)?, settings.source_directory.as_deref().unwrap_or_else(|| Path::new("config")))?; }
                }
            }
            Migrator::up(&db, None).await.map_err(database_error)?;
            for (uri, access) in [(&runtime, Access::Runtime), (&update, Access::Update)] {
                let password = percent_encoding::percent_decode_str(uri.password().ok_or_else(|| error("Database password is required"))?)
                    .decode_utf8().map_err(|_| error("Database password is not valid UTF-8"))?;
                migration::provision::provision(&db, uri.username(), &password, access).await.map_err(database_error)?;
                verify_login(uri, access).await?;
            }
            tracing::info!("Database setup complete; use runtime access for the app and schema-update access for upgrades");
            Ok(())
        }.await;
        db.close().await.map_err(database_error)?;
        result.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bootstrap() -> Url {
        Url::parse("postgres://installer:bootstrap-only@localhost:5432/medtracker").unwrap()
    }

    #[test]
    fn generated_logins_have_distinct_names_and_strong_passwords() {
        let runtime = credential(None, None, &bootstrap(), "runtime").unwrap();
        let update = credential(None, None, &bootstrap(), "update").unwrap();
        assert_ne!(runtime.username(), update.username());
        assert_ne!(runtime.username(), bootstrap().username());
        assert_eq!(runtime.password().unwrap().len(), 64);
        assert_ne!(runtime.password(), update.password());
        assert_ne!(
            credential(None, None, &bootstrap(), "runtime")
                .unwrap()
                .password(),
            runtime.password()
        );
    }

    #[test]
    fn supplied_credentials_are_preserved_and_cross_database_access_is_rejected() {
        let supplied = "postgres://runtime:a%27b%3A%2F%2B%40@localhost:5432/medtracker";
        let uri = credential(Some(supplied), None, &bootstrap(), "runtime").unwrap();
        assert_eq!(uri.as_str(), supplied);
        assert!(
            credential(
                Some("postgres://runtime:password@localhost:5432/other"),
                None,
                &bootstrap(),
                "runtime"
            )
            .is_err()
        );
        assert!(
            credential(
                Some("postgres://runtime:password@other:5432/medtracker"),
                None,
                &bootstrap(),
                "runtime"
            )
            .is_err()
        );
    }

    #[test]
    fn persisted_credentials_are_reused_and_not_silently_rotated() {
        let mut database = DatabaseAccess::default();
        let uri = credential(None, None, &bootstrap(), "runtime").unwrap();
        database.uri = uri.to_string();
        let saved = Overlay {
            database,
            queue: None,
            settings: serde_json::json!({"database_setup":{"stored_database_url":uri.as_str()}}),
        };
        assert_eq!(
            credential(None, Some(&saved), &bootstrap(), "runtime")
                .unwrap()
                .as_str(),
            uri.as_str()
        );
        assert!(
            credential(
                Some("postgres://other:new@localhost:5432/medtracker"),
                Some(&saved),
                &bootstrap(),
                "runtime"
            )
            .is_err()
        );
    }

    #[test]
    fn generated_overlay_is_private_and_does_not_persist_bootstrap_credentials() {
        let source = tempfile::tempdir().unwrap();
        let original = std::fs::read_to_string("config/production.yaml").unwrap();
        let configured = original.replace(
            "host: <%= get_env(name=\"MEDTRACKER_PUBLIC_HOST\", default=\"http://localhost\") %>",
            "host: https://setup-config.example.test",
        );
        assert_ne!(configured, original);
        std::fs::write(source.path().join("production.yaml"), configured).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("runtime/production.local.yaml");
        let mut config = Config::new(&loco_rs::environment::Environment::Production).unwrap();
        config.database.uri = bootstrap().to_string();
        let uri = credential(None, None, &bootstrap(), "runtime").unwrap();
        let overlay = output_config(&config, &uri, None).unwrap();
        persist(&path, &overlay, source.path()).unwrap();
        let original = std::fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&original).contains("bootstrap-only"));
        assert!(persist(&path, &overlay, source.path()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let loaded = Config::from_folder(
            &loco_rs::environment::Environment::Production,
            path.parent().unwrap(),
        )
        .unwrap();
        assert_eq!(loaded.server.host, "https://setup-config.example.test");
        assert!(
            !loaded.settings.unwrap()["browser_session"]["key"]
                .as_str()
                .unwrap()
                .contains("get_env")
        );
        assert_eq!(
            saved_config(&path).unwrap().unwrap().settings["database_setup"]["stored_database_url"],
            uri.as_str()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(saved_config(&path).is_err());
        }
    }
}
