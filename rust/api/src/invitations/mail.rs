use super::*;

#[derive(Clone)]
pub(crate) struct MailConfig {
    smtp_host: Option<String>,
    smtp_port: u16,
    smtp_username: Option<String>,
    smtp_password: Option<String>,
    authentication: String,
    starttls: bool,
    app_url: Url,
    from: Mailbox,
}

pub(super) fn smtp_send(config: MailConfig, email: String, token: String) -> Result<(), ()> {
    let host = config.smtp_host.ok_or(())?;
    let recipient = email.parse::<Mailbox>().map_err(|_| ())?;
    let url = format!(
        "{}/invitations/accept?token={token}",
        config.app_url.as_str().trim_end_matches('/')
    );
    let message = Message::builder()
        .from(config.from)
        .to(recipient)
        .subject("MedTracker invitation")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("You have been invited to MedTracker.\n\nAccept invitation:\n{url}\n\nThis invitation expires in seven days.\n"))
        .map_err(|_| ())?;
    let mut builder = if config.starttls {
        SmtpTransport::starttls_relay(&host).map_err(|_| ())?
    } else {
        SmtpTransport::builder_dangerous(&host)
    }
    .port(config.smtp_port)
    .timeout(Some(StdDuration::from_secs(5)));
    if let (Some(username), Some(password)) = (config.smtp_username, config.smtp_password) {
        let mechanism = match config.authentication.as_str() {
            "login" => Mechanism::Login,
            "plain" => Mechanism::Plain,
            _ => return Err(()),
        };
        builder = builder
            .credentials(Credentials::new(username, password))
            .authentication(vec![mechanism]);
    }
    builder.build().send(&message).map_err(|_| ())?;
    Ok(())
}

impl MailConfig {
    pub(crate) fn from_env() -> Result<Self, String> {
        let smtp_host = std::env::var("SMTP_ADDRESS")
            .ok()
            .filter(|value| !value.is_empty());
        let smtp_port = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_owned())
            .parse::<u16>()
            .map_err(|_| "SMTP_PORT must be a valid port".to_owned())?;
        let smtp_username = std::env::var("SMTP_USER_NAME")
            .ok()
            .filter(|value| !value.is_empty());
        let smtp_password = std::env::var("SMTP_PASSWORD")
            .ok()
            .filter(|value| !value.is_empty());
        if smtp_username.is_some() != smtp_password.is_some() {
            return Err("SMTP credentials must be configured together".to_owned());
        }
        let authentication =
            std::env::var("SMTP_AUTHENTICATION").unwrap_or_else(|_| "plain".to_owned());
        if !matches!(authentication.as_str(), "plain" | "login") {
            return Err("SMTP_AUTHENTICATION must be plain or login".to_owned());
        }
        let starttls = match std::env::var("SMTP_STARTTLS")
            .unwrap_or_else(|_| "true".to_owned())
            .as_str()
        {
            "true" => true,
            "false" => false,
            _ => return Err("SMTP_STARTTLS must be true or false".to_owned()),
        };
        let app_url = Url::parse(
            &std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_owned()),
        )
        .map_err(|_| "APP_URL must be an absolute URL".to_owned())?;
        if !matches!(app_url.scheme(), "http" | "https")
            || app_url.host().is_none()
            || !app_url.username().is_empty()
            || app_url.password().is_some()
            || app_url.query().is_some()
            || app_url.fragment().is_some()
        {
            return Err("APP_URL must be an HTTP origin or base path".to_owned());
        }
        let from = std::env::var("MAILER_FROM")
            .unwrap_or_else(|_| "MedTracker <noreply@medtracker.app>".to_owned())
            .parse::<Mailbox>()
            .map_err(|_| "MAILER_FROM must be a mailbox".to_owned())?;
        if smtp_host
            .as_ref()
            .is_some_and(|value| value.contains(['\r', '\n']))
        {
            return Err("Invalid invitation mail configuration".to_owned());
        }
        Ok(Self {
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            authentication,
            starttls,
            app_url,
            from,
        })
    }
}
