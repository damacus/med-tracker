use serde_json::Value;
use std::sync::OnceLock;

pub fn contract() -> &'static Value {
    static CONTRACT: OnceLock<Value> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml"))
            .expect("valid OpenAPI document")
    })
}

pub fn resolve<'a>(contract: &'a Value, node: &'a Value) -> &'a Value {
    let pointer = node["$ref"]
        .as_str()
        .expect("schema $ref")
        .strip_prefix('#')
        .expect("local $ref");
    contract.pointer(pointer).expect("resolvable $ref")
}

fn hyphenated_uuid(text: &str) -> bool {
    text.len() == 36
        && text.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

pub fn assert_value(contract: &Value, schema: &Value, value: &Value, label: &str) {
    let schema = if schema.get("$ref").is_some() {
        resolve(contract, schema)
    } else {
        schema
    };
    if value.is_null() {
        assert_eq!(schema["nullable"], true, "{label} null but not nullable");
        return;
    }
    if let Some(allowed) = schema["enum"].as_array() {
        assert!(allowed.contains(value), "{label} not in documented enum");
    }
    if let Some(all_of) = schema["allOf"].as_array() {
        for (index, part) in all_of.iter().enumerate() {
            assert_value(contract, part, value, &format!("{label}.allOf[{index}]"));
        }
        if schema.get("type").is_none() {
            return;
        }
    }
    match schema["type"].as_str().expect("schema type") {
        "object" => {
            let object = value
                .as_object()
                .unwrap_or_else(|| panic!("{label} object"));
            for name in schema["required"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                assert!(object.contains_key(name), "{label} missing {name}");
            }
            let properties = schema["properties"].as_object();
            if schema["additionalProperties"] == Value::Bool(false) {
                for key in object.keys() {
                    assert!(
                        properties.is_some_and(|declared| declared.contains_key(key)),
                        "{label} undeclared {key}"
                    );
                }
            }
            for (key, field) in object {
                if let Some(property) = properties.and_then(|declared| declared.get(key)) {
                    assert_value(contract, property, field, &format!("{label}.{key}"));
                } else if schema["additionalProperties"].is_object() {
                    assert_value(
                        contract,
                        &schema["additionalProperties"],
                        field,
                        &format!("{label}.{key}"),
                    );
                }
            }
        }
        "array" => {
            let items = value.as_array().unwrap_or_else(|| panic!("{label} array"));
            if let Some(minimum) = schema["minItems"].as_u64() {
                assert!(items.len() >= minimum as usize, "{label} below minItems");
            }
            if let Some(maximum) = schema["maxItems"].as_u64() {
                assert!(items.len() <= maximum as usize, "{label} above maxItems");
            }
            if schema["uniqueItems"] == Value::Bool(true) {
                let unique = items.iter().collect::<std::collections::HashSet<_>>();
                assert_eq!(unique.len(), items.len(), "{label} not unique");
            }
            for (index, item) in items.iter().enumerate() {
                assert_value(
                    contract,
                    &schema["items"],
                    item,
                    &format!("{label}[{index}]"),
                );
            }
        }
        "integer" => {
            let number = value.as_i64().unwrap_or_else(|| panic!("{label} integer"));
            if let Some(minimum) = schema["minimum"].as_i64() {
                assert!(number >= minimum, "{label} below minimum");
            }
            if let Some(maximum) = schema["maximum"].as_i64() {
                assert!(number <= maximum, "{label} above maximum");
            }
        }
        "boolean" => assert!(value.is_boolean(), "{label} boolean"),
        "string" => {
            let text = value.as_str().unwrap_or_else(|| panic!("{label} string"));
            if let Some(length) = schema["minLength"].as_u64() {
                assert!(
                    text.chars().count() >= length as usize,
                    "{label} below minLength"
                );
            }
            if let Some(length) = schema["maxLength"].as_u64() {
                assert!(
                    text.chars().count() <= length as usize,
                    "{label} above maxLength"
                );
            }
            match schema["format"].as_str() {
                Some("uuid") => assert!(
                    hyphenated_uuid(text) && uuid::Uuid::parse_str(text).is_ok(),
                    "{label} uuid format"
                ),
                Some("date-time") => assert!(
                    chrono::DateTime::parse_from_rfc3339(text).is_ok(),
                    "{label} date-time format"
                ),
                Some("date") => assert!(
                    chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok(),
                    "{label} date format"
                ),
                Some("email") => assert!(
                    text.matches('@').count() == 1
                        && !text.starts_with('@')
                        && !text.ends_with('@'),
                    "{label} email format"
                ),
                Some("uri") => assert!(
                    iri_string::types::UriStr::new(text).is_ok(),
                    "{label} uri format"
                ),
                _ => {}
            }
            if let Some(pattern) = schema["pattern"].as_str() {
                let pattern = regex::Regex::new(pattern)
                    .unwrap_or_else(|error| panic!("{label} invalid documented pattern: {error}"));
                assert!(
                    pattern.is_match(text),
                    "{label} must match documented pattern"
                );
            }
        }
        other => panic!("{label}: unhandled schema type {other}"),
    }
}
