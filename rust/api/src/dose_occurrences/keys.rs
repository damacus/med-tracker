use super::*;

type HmacSha256 = Hmac<Sha256>;

pub(super) fn key(
    secret: &Arc<[u8]>,
    source: &Source,
    window_start: NaiveDate,
    position: i32,
) -> String {
    let payload = json!([
        source.kind().name(),
        source.portable_id(),
        window_start.to_string(),
        position
    ]);
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
    if values.len() != 4 || values[0] != source.kind().name() || values[1] != source.portable_id() {
        return None;
    }
    let date = date(values[2].as_str()?)?;
    let position = values[3]
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())?;
    (position > 0).then_some((date, position))
}
