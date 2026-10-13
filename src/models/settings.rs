use serde_json::Value;

pub(crate) fn node<'a>(settings: Option<&'a Value>, path: &[&str]) -> Option<&'a Value> {
    let mut current = settings?;
    for key in path {
        current = current.get(*key)?;
        if current.is_null() {
            return None;
        }
    }
    Some(current)
}

pub(crate) fn text<'a>(settings: Option<&'a Value>, path: &[&str]) -> Option<&'a str> {
    node(settings, path)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(crate) fn truthy(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::String(value) => {
            let value = value.trim().to_ascii_lowercase();
            ["1", "true", "yes", "on"]
                .contains(&value.as_str())
                .then_some(true)
                .or_else(|| {
                    ["0", "f", "false", "no", "off"]
                        .contains(&value.as_str())
                        .then_some(false)
                })
        }
        _ => None,
    }
}

pub(crate) fn flag(settings: Option<&Value>, path: &[&str]) -> Option<bool> {
    truthy(node(settings, path)?)
}

pub(crate) fn integer(settings: Option<&Value>, path: &[&str]) -> Option<i64> {
    match node(settings, path)? {
        Value::Number(number) => number.as_i64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn settings() -> Value {
        json!({
            "section": {
                "name": " value ",
                "empty": "",
                "on": true,
                "off": false,
                "on_text": "yes",
                "off_text": "0",
                "count": 7,
                "count_text": "30",
                "nullish": null,
                "nested": {"deep": "x", "nothing": null}
            }
        })
    }

    #[test]
    fn text_resolves_nested_non_empty_strings() {
        let value = settings();
        assert_eq!(text(Some(&value), &["section", "name"]), Some("value"));
        assert_eq!(text(Some(&value), &["section", "empty"]), None);
        assert_eq!(text(Some(&value), &["section", "missing"]), None);
        assert_eq!(text(Some(&value), &["missing"]), None);
        assert_eq!(text(None, &["section", "name"]), None);
        assert_eq!(
            text(Some(&value), &["section", "nested", "deep"]),
            Some("x")
        );
    }

    #[test]
    fn flag_accepts_bools_and_env_style_strings() {
        let value = settings();
        assert_eq!(flag(Some(&value), &["section", "on"]), Some(true));
        assert_eq!(flag(Some(&value), &["section", "off"]), Some(false));
        assert_eq!(flag(Some(&value), &["section", "on_text"]), Some(true));
        assert_eq!(flag(Some(&value), &["section", "off_text"]), Some(false));
        assert_eq!(flag(Some(&value), &["section", "empty"]), None);
        assert_eq!(flag(Some(&value), &["section", "name"]), None);
        assert_eq!(flag(Some(&value), &["section", "missing"]), None);
    }

    #[test]
    fn integer_accepts_numbers_and_numeric_strings() {
        let value = settings();
        assert_eq!(integer(Some(&value), &["section", "count"]), Some(7));
        assert_eq!(integer(Some(&value), &["section", "count_text"]), Some(30));
        assert_eq!(integer(Some(&value), &["section", "name"]), None);
        assert_eq!(integer(Some(&value), &["section", "missing"]), None);
    }

    #[test]
    fn null_nodes_resolve_as_absent() {
        let value = settings();
        assert_eq!(node(Some(&value), &["section", "nullish"]), None);
        assert_eq!(node(Some(&value), &["section", "nested", "nothing"]), None);
        assert_eq!(text(Some(&value), &["section", "nullish"]), None);
        assert_eq!(flag(Some(&value), &["section", "nullish"]), None);
        assert_eq!(integer(Some(&value), &["section", "nullish"]), None);
    }
}
