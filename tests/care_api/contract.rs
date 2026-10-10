use serde_json::Value;

pub fn resolve<'a>(contract: &'a Value, node: &'a Value) -> &'a Value {
    let pointer = node["$ref"]
        .as_str()
        .expect("schema $ref")
        .strip_prefix('#')
        .expect("local $ref");
    contract.pointer(pointer).expect("resolvable $ref")
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
    match schema["type"].as_str().expect("schema type") {
        "object" => {
            let object = value
                .as_object()
                .unwrap_or_else(|| panic!("{label} object"));
            if let Some(properties) = schema["properties"].as_object() {
                for name in schema["required"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    assert!(object.contains_key(name), "{label} missing {name}");
                }
                if schema["additionalProperties"] == Value::Bool(false) {
                    for key in object.keys() {
                        assert!(properties.contains_key(key), "{label} undeclared {key}");
                    }
                }
                for (key, field) in object {
                    if let Some(property) = properties.get(key) {
                        assert_value(contract, property, field, &format!("{label}.{key}"));
                    }
                }
            }
            let additional = &schema["additionalProperties"];
            if additional.is_object() {
                for (key, field) in object {
                    assert_value(contract, additional, field, &format!("{label}.{key}"));
                }
            }
        }
        "array" => {
            for (index, item) in value
                .as_array()
                .unwrap_or_else(|| panic!("{label} array"))
                .iter()
                .enumerate()
            {
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
        }
        "boolean" => assert!(value.is_boolean(), "{label} boolean"),
        "string" => {
            let text = value.as_str().unwrap_or_else(|| panic!("{label} string"));
            if let Some(length) = schema["minLength"].as_u64() {
                assert!(text.len() >= length as usize, "{label} below minLength");
            }
            match schema["format"].as_str() {
                Some("uuid") => assert!(uuid::Uuid::parse_str(text).is_ok(), "{label} uuid format"),
                Some("date-time") => assert!(
                    chrono::DateTime::parse_from_rfc3339(text).is_ok(),
                    "{label} date-time format"
                ),
                Some("date") => assert!(
                    chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok(),
                    "{label} date format"
                ),
                _ => {}
            }
            if let Some(pattern) = schema["pattern"].as_str() {
                match pattern {
                    r"^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$" => {
                        let bytes = text.as_bytes();
                        assert!(
                            bytes.len() == 8
                                && bytes[2] == b':'
                                && bytes[5] == b':'
                                && bytes
                                    .iter()
                                    .enumerate()
                                    .all(|(index, byte)| [2, 5].contains(&index)
                                        || byte.is_ascii_digit())
                                && text[0..2].parse::<u8>().is_ok_and(|hour| hour <= 23)
                                && text[3..5].parse::<u8>().is_ok_and(|minute| minute <= 59)
                                && text[6..8].parse::<u8>().is_ok_and(|second| second <= 59),
                            "{label} must match documented time pattern"
                        );
                    }
                    r"^(?:[1-9][0-9]*|[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12})$" =>
                    {
                        let numeric = !text.starts_with('0')
                            && text.bytes().all(|byte| byte.is_ascii_digit());
                        let portable = uuid::Uuid::parse_str(text)
                            .is_ok_and(|id| id.get_variant() == uuid::Variant::RFC4122);
                        assert!(
                            numeric || portable,
                            "{label} must match documented identifier pattern"
                        );
                    }
                    other => panic!("{label}: unhandled pattern {other}"),
                }
            }
        }
        other => panic!("{label}: unhandled schema type {other}"),
    }
}
