use better_auth::AuthResult;
use loco_rs::{
    bgworker::BackgroundWorker,
    mailer::{DEFAULT_MAILER_PRIORITY, Email, MailerWorker},
};
use sea_orm::ConnectionTrait;

use super::policy::{database_error, statement};

pub(super) fn sender(settings: Option<&serde_json::Value>) -> Option<String> {
    crate::models::settings::text(settings, &["mail", "from"]).map(str::to_owned)
}

pub(super) async fn notice_address_in(
    transaction: &impl ConnectionTrait,
    address: &str,
    subject: &str,
    body: &str,
    settings: Option<&serde_json::Value>,
) -> AuthResult<()> {
    let email = Email {
        from: sender(settings),
        to: address.into(),
        subject: subject.into(),
        text: body.into(),
        ..Default::default()
    };
    transaction.execute_raw(statement("INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)", [ulid::Ulid::generate().to_string().into(), serde_json::to_value(email)?.into(), MailerWorker::class_name().into(), DEFAULT_MAILER_PRIORITY.into()])).await.map_err(database_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use loco_rs::{
        config::Config,
        environment::Environment,
        mailer::{Email, EmailSender},
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    #[tokio::test]
    async fn production_smtp_requires_starttls() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::new(&Environment::Production)?;
        let mut smtp = config
            .mailer
            .and_then(|mailer| mailer.smtp)
            .ok_or("SMTP config missing")?;
        assert!(matches!(
            smtp.tls_mode(),
            loco_rs::config::MailerTls::Starttls
        ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        smtp.host = "127.0.0.1".into();
        smtp.port = listener.local_addr()?.port();
        smtp.enable = true;
        smtp.auth = None;
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await?;
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            writer.write_all(b"220 fixture ESMTP\r\n").await?;
            let mut command = String::new();
            reader.read_line(&mut command).await?;
            assert!(
                command.starts_with("EHLO "),
                "Expected SMTP greeting before TLS"
            );
            writer.write_all(b"250-fixture\r\n250 STARTTLS\r\n").await?;
            command.clear();
            reader.read_line(&mut command).await?;
            assert_eq!(command, "STARTTLS\r\n");
            writer.write_all(b"454 TLS upgrade unavailable\r\n").await?;
            command.clear();
            reader.read_line(&mut command).await?;
            assert!(
                command.is_empty() || command == "QUIT\r\n",
                "SMTP must not continue in cleartext after TLS is refused"
            );
            Ok::<_, std::io::Error>(())
        });
        let result = EmailSender::smtp(&smtp)?
            .mail(&Email {
                from: Some("sender@example.invalid".into()),
                to: "recipient@example.invalid".into(),
                subject: "Synthetic TLS check".into(),
                text: "This message must not be delivered".into(),
                ..Default::default()
            })
            .await;
        assert!(result.is_err(), "SMTP must fail when STARTTLS is refused");
        tokio::time::timeout(std::time::Duration::from_secs(5), server).await???;
        Ok(())
    }
}
