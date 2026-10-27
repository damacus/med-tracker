use super::contract::*;
use serde_json::{Value, json};

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
    let schema = json!({"type":"string","pattern":"^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$"});
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

#[test]
fn contract_helper_accepts_absolute_rfc3986_uris() {
    let document = document();
    let schema = json!({"type":"string","format":"uri"});
    for value in [
        "https://medtracker.example/.well-known/oauth-authorization-server",
        "io.damacus.medtracker:/oauth2redirect",
        "urn:medtracker:task%20one",
    ] {
        assert_value(&document, &schema, &json!(value), "value");
    }
}

#[test]
fn contract_helper_rejects_non_uri_strings() {
    let document = document();
    let schema = json!({"type":"string","format":"uri"});
    for value in [
        "/relative/path",
        "https://example.test/has space",
        "https://example.test/%GG",
        "https://example.test/é",
    ] {
        let panic = std::panic::catch_unwind(|| {
            assert_value(&document, &schema, &json!(value), "value");
        })
        .unwrap_err();
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(message.contains("value uri format"), "{value}: {message}");
    }
}
