use loco_rs::config::{Config, QueueConfig};
use serde::Deserialize;
use url::Url;

#[derive(Debug)]
pub struct ConnectionError(&'static str);

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ConnectionError {}

impl From<ConnectionError> for loco_rs::Error {
    fn from(value: ConnectionError) -> Self {
        Self::string(value.0)
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FieldValue {
    Literal(String),
    Deferred { template: String },
}

#[derive(Default)]
struct ConnectionField(Option<String>);

impl<'de> Deserialize<'de> for ConnectionField {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let value = match Option::<FieldValue>::deserialize(deserializer)? {
            None => None,
            Some(FieldValue::Literal(value)) => Some(value),
            Some(FieldValue::Deferred { template }) => Some(
                loco_rs::controller::views::template(&template, serde_json::json!({}))
                    .map_err(|_| D::Error::custom("Cannot resolve PostgreSQL connection field"))?,
            ),
        };
        Ok(Self(value))
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub(super) struct Credentials {
    username: ConnectionField,
    password: ConnectionField,
}

impl Credentials {
    pub(super) fn connection_uri(&self, endpoint: &Url) -> Result<Option<Url>, ConnectionError> {
        match (present(&self.username), present(&self.password)) {
            (None, None) => Ok(None),
            (Some(username), Some(password)) => {
                if !username
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                {
                    return Err(ConnectionError(
                        "PostgreSQL login names must contain only ASCII letters, digits, underscores or hyphens",
                    ));
                }
                let mut uri = endpoint.clone();
                let password = percent_encoding::utf8_percent_encode(
                    password,
                    percent_encoding::NON_ALPHANUMERIC,
                );
                uri.set_username(username)
                    .map_err(|()| invalid_credentials())?;
                uri.set_password(Some(&password.to_string()))
                    .map_err(|()| invalid_credentials())?;
                Ok(Some(uri))
            }
            _ => Err(invalid_credentials()),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Postgres {
    url: ConnectionField,
    host: ConnectionField,
    port: ConnectionField,
    dbname: ConnectionField,
    #[serde(flatten)]
    credentials: Credentials,
}

fn present(value: &ConnectionField) -> Option<&str> {
    value.0.as_deref().filter(|value| !value.is_empty())
}

fn invalid_credentials() -> ConnectionError {
    ConnectionError("PostgreSQL username and password fields must both be supplied")
}

pub fn configure(config: &mut Config) -> Result<(), ConnectionError> {
    let fields = config
        .settings
        .as_ref()
        .and_then(|settings| settings.get("postgres"));
    let Some(fields) = fields.filter(|fields| !fields.is_null()) else {
        return Ok(());
    };
    let fields: Postgres = serde_json::from_value(fields.clone())
        .map_err(|_| ConnectionError("Invalid PostgreSQL connection fields"))?;
    if present(&fields.url).is_some()
        || [
            &fields.host,
            &fields.port,
            &fields.dbname,
            &fields.credentials.username,
            &fields.credentials.password,
        ]
        .into_iter()
        .all(|field| present(field).is_none())
    {
        return Ok(());
    }
    let host = present(&fields.host).ok_or(ConnectionError(
        "PGHOST is required with PostgreSQL connection fields",
    ))?;
    let dbname = present(&fields.dbname).ok_or(ConnectionError(
        "PGDATABASE is required with PostgreSQL connection fields",
    ))?;
    let port = present(&fields.port)
        .map_or(Ok(5432), str::parse::<u16>)
        .map_err(|_| ConnectionError("PGPORT must be a valid PostgreSQL port"))?;
    if port == 0 {
        return Err(ConnectionError("PGPORT must be a valid PostgreSQL port"));
    }
    let mut uri = Url::parse("postgresql://localhost/")
        .map_err(|_| ConnectionError("Cannot construct PostgreSQL connection"))?;
    if let Ok(address) = host.parse::<std::net::IpAddr>() {
        uri.set_ip_host(address)
            .map_err(|()| ConnectionError("Invalid PGHOST"))?;
    } else {
        uri.set_host(Some(host))
            .map_err(|_| ConnectionError("Invalid PGHOST"))?;
    }
    uri.set_port(Some(port))
        .map_err(|()| ConnectionError("Invalid PGPORT"))?;
    uri.path_segments_mut()
        .map_err(|()| ConnectionError("Invalid PGDATABASE"))?
        .clear()
        .push(dbname);
    let uri = fields
        .credentials
        .connection_uri(&uri)?
        .ok_or_else(invalid_credentials)?;
    config.database.uri = uri.to_string();
    if let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() {
        queue.uri.clone_from(&config.database.uri);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use loco_rs::{config::QueueConfig, environment::Environment};
    use serde_json::json;
    use url::Url;

    fn config(fields: serde_json::Value) -> Config {
        let mut config = Config::from_folder(&Environment::Production, "config".as_ref()).unwrap();
        config.database.uri = "postgres://stored:password@stored/database".to_owned();
        config.settings.as_mut().unwrap()["postgres"] = fields;
        config
    }

    #[test]
    fn cnpg_fields_escape_credentials_and_database_names_and_configure_the_queue() {
        let mut config = config(json!({
            "host":"database.home.svc", "port":"5433", "dbname":"care/household?one",
            "username":"med_tracker_runtime", "password":"password:@/?#%2F+with spaces"
        }));
        configure(&mut config).unwrap();
        let uri = Url::parse(&config.database.uri).unwrap();
        assert_eq!(uri.host_str(), Some("database.home.svc"));
        assert_eq!(uri.port(), Some(5433));
        assert_eq!(uri.username(), "med_tracker_runtime");
        assert_eq!(
            percent_encoding::percent_decode_str(uri.password().unwrap())
                .decode_utf8()
                .unwrap(),
            "password:@/?#%2F+with spaces"
        );
        assert_eq!(
            percent_encoding::percent_decode_str(uri.path())
                .decode_utf8()
                .unwrap(),
            "/care/household?one"
        );
        assert_eq!(uri.path_segments().unwrap().count(), 1);
        let Some(QueueConfig::Postgres(queue)) = config.queue else {
            panic!("Postgres queue required")
        };
        assert_eq!(queue.uri, config.database.uri);
    }

    #[test]
    fn cnpg_fields_default_the_port() {
        let mut config = config(json!({
            "host":"database", "dbname":"med-tracker", "username":"runtime", "password":"password"
        }));
        configure(&mut config).unwrap();
        assert_eq!(
            Url::parse(&config.database.uri).unwrap().host_str(),
            Some("database")
        );
        assert_eq!(Url::parse(&config.database.uri).unwrap().port(), Some(5432));
    }

    #[test]
    fn cnpg_fields_accept_ipv6_addresses() {
        let mut config = config(json!({
            "host":"::1", "dbname":"care", "username":"runtime", "password":"password"
        }));
        configure(&mut config).unwrap();
        assert_eq!(
            Url::parse(&config.database.uri).unwrap().host_str(),
            Some("[::1]")
        );
    }

    #[test]
    fn cnpg_fields_credentials_preserve_the_endpoint_and_reject_half_a_pair() {
        let endpoint =
            Url::parse("postgresql://administrator:password@database:5433/care").unwrap();
        let credentials: Credentials = serde_json::from_value(json!({
            "username":"med_tracker_runtime", "password":"password%2F:@\"\\with spaces"
        }))
        .unwrap();
        let uri = credentials.connection_uri(&endpoint).unwrap().unwrap();
        assert_eq!(uri.host_str(), endpoint.host_str());
        assert_eq!(uri.port(), endpoint.port());
        assert_eq!(uri.path(), endpoint.path());
        assert_eq!(uri.username(), "med_tracker_runtime");
        assert_eq!(
            percent_encoding::percent_decode_str(uri.password().unwrap())
                .decode_utf8()
                .unwrap(),
            "password%2F:@\"\\with spaces"
        );
        for fields in [
            json!({"username":"runtime"}),
            json!({"password":"private-password"}),
        ] {
            let credentials: Credentials = serde_json::from_value(fields).unwrap();
            let error = credentials
                .connection_uri(&endpoint)
                .unwrap_err()
                .to_string();
            assert!(!error.contains("private-password"));
        }
        assert!(
            Credentials::default()
                .connection_uri(&endpoint)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn cnpg_fields_literal_passwords_are_not_templates() {
        let password = "{{ get_env(name=\"CNPG_LITERAL_PASSWORD\", default=\"changed\") }}";
        let mut config = config(
            json!({"host":"database", "dbname":"care", "username":"runtime", "password":password}),
        );
        configure(&mut config).unwrap();
        let uri = Url::parse(&config.database.uri).unwrap();
        assert_eq!(
            percent_encoding::percent_decode_str(uri.password().unwrap())
                .decode_utf8()
                .unwrap(),
            password
        );
    }

    #[test]
    fn cnpg_fields_deferred_values_preserve_whitespace_and_literal_templates() {
        let password = " leading \r\n{{ literal template }}\n";
        let mut config = config(json!({
            "host":"database", "dbname":"care", "username":"runtime",
            "password":{"template":format!("{{% raw %}}{password}{{% endraw %}}")}
        }));
        configure(&mut config).unwrap();
        let uri = Url::parse(&config.database.uri).unwrap();
        assert_eq!(
            percent_encoding::percent_decode_str(uri.password().unwrap())
                .decode_utf8()
                .unwrap(),
            password
        );
    }

    #[test]
    fn cnpg_fields_preserve_existing_urls_when_absent_or_explicit() {
        for fields in [
            json!({}),
            serde_json::Value::Null,
            json!({"url":"postgres://explicit" ,"host":"ignored"}),
        ] {
            let mut config = config(fields);
            let original = config.database.uri.clone();
            configure(&mut config).unwrap();
            assert_eq!(config.database.uri, original);
        }
    }

    #[test]
    fn cnpg_fields_reject_partial_or_invalid_fields_without_exposing_values() {
        for fields in [
            json!({"host":"database", "password":"private-password"}),
            json!({"host":"database", "port":"70000", "dbname":"care", "username":"runtime", "password":"private-password"}),
            json!({"host":"bad/host", "dbname":"care", "username":"runtime", "password":"private-password"}),
            json!({"host":"database", "dbname":"care", "username":"care%2Fworker", "password":"private-password"}),
        ] {
            let mut config = config(fields);
            let original = config.database.uri.clone();
            let error = configure(&mut config).unwrap_err().to_string();
            assert!(!error.contains("private-password"));
            assert_eq!(config.database.uri, original);
        }
    }
}
