use super::*;

type HmacSha256 = Hmac<Sha256>;

pub(super) fn key(
    secret: &Arc<[u8]>,
    source: &Source,
    window_start: NaiveDate,
    position: i32,
) -> String {
    let mut payload = vec![
        json!(source.kind().name()),
        json!(source.portable_id()),
        json!(window_start.to_string()),
        json!(position),
    ];
    if let Source::Schedule(row) = source
        && let Some(revision) = row.clinical_revision
    {
        payload.push(json!(revision));
    }
    let payload = Value::Array(payload);
    let encoded = URL_SAFE_NO_PAD.encode(payload.to_string());
    let mut mac = HmacSha256::new_from_slice(secret).expect("validated HMAC secret");
    mac.update(b"medtracker-dose-occurrence-v1\0");
    mac.update(encoded.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    format!("{encoded}.{signature}")
}

pub(super) fn decode_key(
    secret: &Arc<[u8]>,
    source: &Source,
    value: &str,
) -> Option<(NaiveDate, i32)> {
    if value.len() > 1024 {
        return None;
    }
    let (encoded, signature) = value.split_once('.')?;
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    let mut mac = HmacSha256::new_from_slice(secret).ok()?;
    mac.update(b"medtracker-dose-occurrence-v1\0");
    mac.update(encoded.as_bytes());
    mac.verify_slice(&signature).ok()?;
    let payload: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).ok()?).ok()?;
    let values = payload.as_array()?;
    let revision = match source {
        Source::Schedule(row) => row.clinical_revision,
        Source::Assignment(_) => None,
    };
    if values.len() != if revision.is_some() { 5 } else { 4 }
        || values[0] != source.kind().name()
        || values[1] != source.portable_id()
        || revision.is_some_and(|revision| values.get(4).and_then(Value::as_i64) != Some(revision))
    {
        return None;
    }
    let date = date(values[2].as_str()?)?;
    let position = values[3]
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())?;
    (position > 0).then_some((date, position))
}
