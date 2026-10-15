use serde::{
    Serialize, Serializer,
    ser::{Error, SerializeMap, SerializeSeq},
};
use serde_json::Value;

struct TemplateData<'a>(&'a Value);

#[must_use]
pub fn template_data(value: &Value) -> impl Serialize + '_ {
    TemplateData(value)
}

impl Serialize for TemplateData<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::String(value) => serializer.serialize_str(value),
            Value::Number(value) => {
                if let Some(integer) = value.as_i64() {
                    return serializer.serialize_i64(integer);
                }
                if let Some(integer) = value.as_u64() {
                    return serializer.serialize_u64(integer);
                }
                let text = value.as_str();
                if !text.contains(['.', 'e', 'E']) {
                    return Err(S::Error::custom(
                        "Template integer is outside the supported range",
                    ));
                }
                let presentation = value
                    .as_f64()
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| S::Error::custom("Template number is not finite"))?;
                serializer.serialize_f64(presentation)
            }
            Value::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(&template_data(value))?;
                }
                sequence.end()
            }
            Value::Object(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, &template_data(value))?;
                }
                map.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::template_data;
    use loco_rs::controller::views::{ViewRenderer, engines::TeraView};
    use serde_json::json;

    #[test]
    fn tera_nested_numbers_keep_integer_identity_and_presentation_contracts()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        std::fs::write(
            directory.path().join("numbers.html"),
            concat!(
                "{% if selected.member_pages > 1 %}pages{% endif %}|",
                "{{ selected.id }}|",
                "{% if selected.id > 9007199254740992 %}large{% endif %}|",
                "{{ signed }}|{{ unsigned }}|{{ clinical }}|",
                "{{ rows[0].amount }}|{% if not missing %}null{% endif %}|",
                "{{ dangerous }}|{{ ordinary }}|{% if flag %}true{% endif %}"
            ),
        )?;
        let view = TeraView::from_custom_dir(&directory.path(), |_| Ok(()))?;
        let data = json!({
            "selected": {"member_pages": 2, "id": 9_007_199_254_740_993_i64},
            "signed": i64::MIN,
            "unsigned": u64::MAX,
            "clinical": "12345678.90",
            "rows": [{"amount": "0.10"}],
            "missing": null,
            "dangerous": "<strong>&",
            "ordinary": 1.25,
            "flag": true
        });
        let rendered = view.render("numbers.html", template_data(&data))?;
        assert_eq!(
            rendered,
            "pages|9007199254740993|large|-9223372036854775808|18446744073709551615|12345678.90|0.10|null|&lt;strong&gt;&amp;|1.25|true"
        );
        Ok(())
    }

    #[test]
    fn tera_rejects_unsupported_numbers_instead_of_rounding_integer_identity()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        std::fs::write(directory.path().join("numbers.html"), "{{ number }}")?;
        let view = TeraView::from_custom_dir(&directory.path(), |_| Ok(()))?;
        for number in [
            "18446744073709551616",
            "-9223372036854775809",
            "1e400",
            "-1e400",
        ] {
            let data: serde_json::Value =
                serde_json::from_str(&format!("{{\"number\":{number}}}"))?;
            assert!(
                view.render("numbers.html", template_data(&data)).is_err(),
                "Unsupported number rendered: {number}"
            );
        }
        Ok(())
    }
}
