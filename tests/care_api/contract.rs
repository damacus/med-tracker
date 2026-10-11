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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document() -> Value {
        json!({})
    }

    #[test]
    fn contract_helper_accepts_nested_objects_and_all_of() {
        let document = document();
        let schema = json!({
            "type":"object",
            "allOf":[{"type":"object","required":["id"],"properties":{"id":{"type":"integer","minimum":1,"maximum":5}}}],
            "required":["id","name"],
            "additionalProperties":false,
            "properties":{"id":{"type":"integer"},"name":{"type":"string","minLength":1}}
        });
        assert_value(
            &document,
            &schema,
            &json!({"id":3,"name":"synthetic"}),
            "value",
        );
    }

    #[test]
    fn contract_helper_applies_additional_properties_schema_only_to_undeclared_keys() {
        let document = document();
        let schema = json!({
            "type":"object",
            "properties":{"kind":{"type":"string","enum":["fixed"]}},
            "additionalProperties":{"type":"string","minLength":2}
        });
        assert_value(
            &document,
            &schema,
            &json!({"kind":"fixed","extra":"long-enough"}),
            "value",
        );
    }

    #[test]
    fn contract_helper_required_check_does_not_depend_on_properties() {
        let document = document();
        let schema = json!({"type":"object","required":["id"]});
        assert_value(&document, &schema, &json!({"id":1}), "value");
    }

    #[test]
    #[should_panic(expected = "missing id")]
    fn contract_helper_missing_required_without_properties() {
        let document = document();
        let schema = json!({"type":"object","required":["id"]});
        assert_value(&document, &schema, &json!({}), "value");
    }

    #[test]
    #[should_panic(expected = "undeclared extra")]
    fn contract_helper_rejects_undeclared_key_without_properties() {
        let document = document();
        let schema = json!({"type":"object","additionalProperties":false});
        assert_value(&document, &schema, &json!({"extra":1}), "value");
    }

    #[test]
    #[should_panic(expected = "below minItems")]
    fn contract_helper_rejects_min_items() {
        let document = document();
        let schema = json!({"type":"array","minItems":1});
        assert_value(&document, &schema, &json!([]), "value");
    }

    #[test]
    #[should_panic(expected = "above maxItems")]
    fn contract_helper_rejects_max_items() {
        let document = document();
        let schema = json!({"type":"array","maxItems":1});
        assert_value(&document, &schema, &json!([1, 2]), "value");
    }

    #[test]
    #[should_panic(expected = "not unique")]
    fn contract_helper_rejects_duplicate_items() {
        let document = document();
        let schema = json!({"type":"array","uniqueItems":true});
        assert_value(&document, &schema, &json!(["a", "a"]), "value");
    }

    #[test]
    #[should_panic(expected = "above maximum")]
    fn contract_helper_rejects_integer_above_maximum() {
        let document = document();
        let schema = json!({"type":"integer","maximum":100});
        assert_value(&document, &schema, &json!(101), "value");
    }

    #[test]
    #[should_panic(expected = "below minLength")]
    fn contract_helper_counts_min_length_in_characters() {
        let document = document();
        let schema = json!({"type":"string","minLength":3});
        assert_value(&document, &schema, &json!("éx"), "value");
    }

    #[test]
    #[should_panic(expected = "uuid format")]
    fn contract_helper_rejects_simple_form_uuid() {
        let document = document();
        let schema = json!({"type":"string","format":"uuid"});
        assert_value(
            &document,
            &schema,
            &json!("550e8400e29b41d4a716446655440000"),
            "value",
        );
    }

    #[test]
    fn contract_helper_accepts_hyphenated_uuid_and_documented_pattern() {
        let document = document();
        let schema = json!({"type":"string","format":"uuid"});
        assert_value(
            &document,
            &schema,
            &json!("550e8400-e29b-41d4-a716-446655440000"),
            "value",
        );
        let schema = json!({"type":"string","pattern":"^-?[0-9]+(?:\\.[0-9]+)?$"});
        assert_value(&document, &schema, &json!("-10.5"), "value");
    }

    #[test]
    #[should_panic(expected = "must match documented pattern")]
    fn contract_helper_rejects_pattern_mismatch() {
        let document = document();
        let schema =
            json!({"type":"string","pattern":"^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$"});
        assert_value(&document, &schema, &json!("24:00:00"), "value");
    }

    #[test]
    #[should_panic(expected = "email format")]
    fn contract_helper_rejects_invalid_email() {
        let document = document();
        let schema = json!({"type":"string","format":"email"});
        assert_value(
            &document,
            &schema,
            &json!("persistence.example.test"),
            "value",
        );
    }
}
