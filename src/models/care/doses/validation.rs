use super::*;

pub(crate) fn valid_identifier(value: &str) -> bool {
    let numeric = value
        .as_bytes()
        .first()
        .is_some_and(|byte| (b'1'..=b'9').contains(byte))
        && value.bytes().all(|byte| byte.is_ascii_digit());
    numeric
        || (value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                19 => matches!(byte, b'8' | b'9' | b'a' | b'b' | b'A' | b'B'),
                _ => byte.is_ascii_hexdigit(),
            }))
}

pub(crate) fn parse_decimal(value: &Value) -> Option<Decimal> {
    let text = value.as_str()?;
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let mut parts = unsigned.split('.');
    let whole = parts.next()?;
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = parts.next()
        && (fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return None;
    }
    if parts.next().is_some() {
        return None;
    }
    Decimal::from_str_exact(text).ok()
}
