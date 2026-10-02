use chrono::{DateTime, LocalResult, NaiveDateTime, TimeZone, Utc};

pub(super) fn now_local() -> String {
    let timezone = configured_timezone();
    Utc::now()
        .with_timezone(&timezone)
        .format("%Y-%m-%dT%H:%M")
        .to_string()
}

pub(super) fn taken_at(value: &str) -> Option<String> {
    let time = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").ok()?;
    let timezone = configured_timezone();
    let time = match timezone.from_local_datetime(&time) {
        LocalResult::Single(value) => value,
        _ => return None,
    };
    Some(time.to_rfc3339())
}

pub(super) fn configured_timezone() -> chrono_tz::Tz {
    std::env::var("TZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chrono_tz::UTC)
}

pub(crate) fn dashboard_now() -> DateTime<Utc> {
    if cfg!(debug_assertions) && std::env::var_os("CONTRACT_PROJECT").is_some() {
        if let Some(now) = std::env::var("CONTRACT_DASHBOARD_NOW")
            .ok()
            .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        {
            return now.with_timezone(&Utc);
        }
    }
    Utc::now()
}

pub(super) fn dashboard_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

#[cfg(test)]
pub(super) fn is_today_in_zone(
    value: &str,
    today: chrono::NaiveDate,
    timezone: chrono_tz::Tz,
) -> bool {
    dashboard_time(value)
        .is_some_and(|timestamp| timestamp.with_timezone(&timezone).date_naive() == today)
}
