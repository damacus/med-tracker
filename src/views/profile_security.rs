use crate::models::{
    identity::better_auth::security::{
        SecurityKey, SecurityOverview, SecurityPasskey, SecuritySessionId,
    },
    ids::AccountId,
};
use chrono::{DateTime, Datelike, SecondsFormat, Utc};
use chrono_tz::Tz;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(crate) struct SessionTime {
    datetime: String,
    label: String,
    exact: String,
}

#[derive(Serialize)]
struct Session<'a> {
    id: &'a SecuritySessionId,
    current: bool,
    created: SessionTime,
    expires: SessionTime,
}

#[derive(Serialize)]
pub(crate) struct Security<'a> {
    account_id: AccountId,
    totp: bool,
    password: bool,
    zitadel: bool,
    provider_available: bool,
    can_unlink_provider: bool,
    passkey_count: String,
    keys: &'a [SecurityKey],
    passkeys: &'a [SecurityPasskey],
    sessions: Vec<Session<'a>>,
    session_zone: &'static str,
}

pub(crate) fn context<'a>(
    overview: &'a SecurityOverview,
    zone: Tz,
    labels: &Value,
    now: DateTime<Utc>,
) -> Security<'a> {
    Security {
        account_id: overview.account_id,
        totp: overview.totp,
        password: overview.password,
        zitadel: overview.zitadel,
        provider_available: overview.provider_available,
        can_unlink_provider: overview.can_unlink_provider,
        passkey_count: overview.passkey_count.to_string(),
        keys: &overview.keys,
        passkeys: &overview.passkeys,
        session_zone: zone.name(),
        sessions: overview
            .sessions
            .iter()
            .map(|session| Session {
                id: &session.id,
                current: session.current,
                created: session_time(session.created, zone, labels, now),
                expires: session_time(session.expires, zone, labels, now),
            })
            .collect(),
    }
}

fn session_time(value: DateTime<Utc>, zone: Tz, labels: &Value, now: DateTime<Utc>) -> SessionTime {
    let local = value.with_timezone(&zone);
    let time = local.format("%H:%M").to_string();
    let label = if local.date_naive() == now.with_timezone(&zone).date_naive() {
        labels["profiles"]["security"]["session_today"]
            .as_str()
            .unwrap_or_default()
            .replace("%{time}", &time)
    } else {
        let report = &labels["reports"]["medication_review"];
        let month = report["months"][local.month0() as usize]
            .as_str()
            .unwrap_or_default();
        let date = report["date_format"]
            .as_str()
            .unwrap_or_default()
            .replace("%{day}", &local.day().to_string())
            .replace("%{month}", month)
            .replace("%{year}", &local.year().to_string());
        labels["profiles"]["security"]["session_at"]
            .as_str()
            .unwrap_or_default()
            .replace("%{date}", &date)
            .replace("%{time}", &time)
    };
    SessionTime {
        datetime: value.to_rfc3339_opts(SecondsFormat::AutoSi, true),
        label,
        exact: local.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("fixed RFC3339 date")
            .with_timezone(&Utc)
    }

    fn labels(locale: &str) -> Value {
        crate::models::care::report_pdf::translations(locale).expect("profile translations")
    }

    #[test]
    fn today_uses_the_account_calendar_day_and_preserves_the_exact_instant() {
        let rendered = session_time(
            instant("2026-11-08T23:30:12.345Z"),
            chrono_tz::America::New_York,
            &labels("en"),
            instant("2026-11-09T02:00:00Z"),
        );
        assert_eq!(rendered.label, "Today at 18:30");
        assert_eq!(rendered.datetime, "2026-11-08T23:30:12.345Z");
        assert_eq!(rendered.exact, "2026-11-08T18:30:12.345-05:00");
    }

    #[test]
    fn daylight_saving_overlap_keeps_distinct_semantic_times() {
        let now = instant("2026-10-25T12:00:00Z");
        let first = session_time(
            instant("2026-10-25T00:30:00Z"),
            chrono_tz::Europe::London,
            &labels("en"),
            now,
        );
        let second = session_time(
            instant("2026-10-25T01:30:00Z"),
            chrono_tz::Europe::London,
            &labels("en"),
            now,
        );
        assert_eq!(first.label, "Today at 01:30");
        assert_eq!(second.label, first.label);
        assert_ne!(first.datetime, second.datetime);
        assert!(first.exact.ends_with("+01:00"));
        assert!(second.exact.ends_with("+00:00"));
    }

    #[test]
    fn every_locale_formats_a_full_date_in_the_account_timezone() {
        for (locale, expected) in [
            ("en", "9 November 2026 at 08:58"),
            ("es", "9 de noviembre de 2026 a las 08:58"),
            ("cy", "9 Tachwedd 2026 am 08:58"),
            ("ga", "9 Samhain 2026 ag 08:58"),
            ("pt", "9 de novembro de 2026 às 08:58"),
        ] {
            let rendered = session_time(
                instant("2026-11-09T13:58:00Z"),
                chrono_tz::America::New_York,
                &labels(locale),
                instant("2026-11-10T13:00:00Z"),
            );
            assert_eq!(rendered.label, expected, "{locale}");
            assert_eq!(rendered.datetime, "2026-11-09T13:58:00Z");
        }
    }
}
