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
