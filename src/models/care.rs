pub mod administration;
pub mod ai_suggestions;
pub mod assignments;
pub mod browser_push;
pub mod browser_query;
pub mod browser_treatments;
pub mod dashboard;
pub mod data_exports;
pub mod dosages;
pub mod dose_history;
pub mod dose_occurrences;
pub mod doses;
pub mod global_search;
pub mod guardian_declarations;
pub mod health_events;
pub mod invitations;
pub mod locations;
pub mod medication_lookup;
pub mod medications;
pub mod native_device_tokens;
pub mod orders;
pub mod pause_periods;
pub mod people;
pub mod portable_crypto;
pub mod portable_imports;
pub mod profile_export;
pub mod push_subscriptions;
pub mod report_pdf;
pub mod reports;
pub mod review_prompts;
pub mod stock_check;
pub mod sync;
pub(crate) mod treatment_command;
pub mod treatments;
pub mod ui_capabilities;

pub mod care_warning;

pub mod person_carers;

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

#[cfg(test)]
mod tests {
    #[test]
    fn valid_identifier_matches_the_documented_resource_identifier_contract() {
        for accepted in [
            "1",
            "90001",
            "0199a4b5-c6d7-4e9f-8012-3456789abcde",
            "0199A4B5-C6D7-4E9F-8012-3456789ABCDE",
        ] {
            assert!(super::valid_identifier(accepted), "rejected {accepted}");
        }
        for rejected in [
            "",
            "0",
            "007",
            "+1",
            "-1",
            "1.0",
            " 1",
            "0199a4b5c6d74e9f80123456789abcde",
            "{0199a4b5-c6d7-4e9f-8012-3456789abcde}",
            "urn:uuid:0199a4b5-c6d7-4e9f-8012-3456789abcde",
            "00000000-0000-0000-0000-000000000000",
            "0199a4b5-c6d7-4e9f-0012-3456789abcde",
            "0199a4b5-c6d7-4e9f-8012-3456789abc",
        ] {
            assert!(!super::valid_identifier(rejected), "accepted {rejected}");
        }
    }
}
