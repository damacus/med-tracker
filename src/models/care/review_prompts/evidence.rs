use crate::models::entities::review_evidence;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::LazyLock;

static TERMINOLOGY: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../assets/reports/terminology/rxclass_terminology.json"
    ))
    .expect("review terminology JSON")
});
static ALIASES: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../assets/reports/terminology/review_terminology_aliases.json"
    ))
    .expect("review terminology aliases JSON")
});

pub(super) struct EvidenceMatch<'a> {
    pub evidence: &'a review_evidence::Model,
    pub primary_is_first: bool,
    pub matched_term: String,
    pub match_type: &'static str,
    pub confidence: String,
    pub instruction: &'static str,
    pub risk: String,
    pub excerpt: String,
    pub reason: String,
}

struct Identity {
    terms: Vec<String>,
    classes: Vec<String>,
}

fn normalize(value: &str) -> String {
    let mut bracketed = String::new();
    let mut inside = false;
    for character in value.chars() {
        match character {
            '[' => {
                inside = true;
                bracketed.push(' ');
            }
            ']' if inside => {
                inside = false;
                bracketed.push(' ');
            }
            _ if !inside => bracketed.push(character),
            _ => {}
        }
    }
    bracketed
        .chars()
        .flat_map(|character| {
            if character.is_alphanumeric() {
                character.to_lowercase().collect::<Vec<_>>()
            } else {
                vec![' ']
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strength(word: &str) -> bool {
    let number = word
        .trim_end_matches(|character: char| character.is_ascii_alphabetic() || character == '%');
    let unit = &word[number.len()..];
    if !matches!(unit, "" | "mg" | "mcg" | "g" | "ml" | "iu" | "%") {
        return false;
    }
    let mut parts = number.split('.');
    let first = parts.next().unwrap_or("");
    let second = parts.next();
    !first.is_empty()
        && first.bytes().all(|byte| byte.is_ascii_digit())
        && second.is_none_or(|digits| {
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        })
        && parts.next().is_none()
}

fn medication_term(value: &str) -> String {
    const DOSAGE: &[&str] = &[
        "capsule",
        "capsules",
        "cream",
        "drops",
        "gel",
        "inhaler",
        "injection",
        "liquid",
        "ointment",
        "pad",
        "pads",
        "powder",
        "sachet",
        "sachets",
        "spray",
        "sprays",
        "suspension",
        "syrup",
        "tablet",
        "tablets",
    ];
    normalize(value)
        .split_whitespace()
        .take_while(|word| !DOSAGE.contains(word) && !strength(word))
        .collect::<Vec<_>>()
        .join(" ")
}

fn overlaps(first: &str, second: &str) -> bool {
    !first.is_empty()
        && !second.is_empty()
        && (format!(" {first} ").contains(&format!(" {second} "))
            || format!(" {second} ").contains(&format!(" {first} ")))
}

fn owns(evidence: &review_evidence::Model, name: &str) -> bool {
    let name = medication_term(name);
    evidence
        .candidate_terms
        .iter()
        .any(|term| overlaps(&name, &normalize(term)))
}

fn identity(name: &str, evidence: &[review_evidence::Model]) -> Identity {
    let name_term = medication_term(name);
    let mut terms = vec![name_term.clone()];
    let mut classes = Vec::new();
    for row in evidence.iter().filter(|row| owns(row, name)) {
        terms.extend(row.candidate_terms.iter().map(|term| normalize(term)));
        classes.extend(row.pharmacologic_classes.iter().map(|term| normalize(term)));
    }
    if let Some(entries) = TERMINOLOGY.get("entries").and_then(Value::as_array) {
        for entry in entries {
            let entry_terms: Vec<String> = ["selection_term", "ingredient_name"]
                .iter()
                .filter_map(|field| entry.get(*field).and_then(Value::as_str))
                .map(normalize)
                .collect();
            if !entry_terms.iter().any(|term| overlaps(&name_term, term)) {
                continue;
            }
            terms.extend(entry_terms);
            if let Some(entry_classes) = entry.get("classes").and_then(Value::as_array) {
                for class in entry_classes {
                    let Some(label) = class.get("name").and_then(Value::as_str) else {
                        continue;
                    };
                    let canonical = singular_last_word(&normalize(label));
                    classes.push(canonical.clone());
                    if let Some(groups) = ALIASES.as_array() {
                        for group in groups {
                            if group
                                .get("canonical_class")
                                .and_then(Value::as_str)
                                .map(normalize)
                                .map(|value| singular_last_word(&value))
                                .as_deref()
                                == Some(canonical.as_str())
                                && let Some(words) = group.get("terms").and_then(Value::as_array)
                            {
                                classes
                                    .extend(words.iter().filter_map(Value::as_str).map(normalize));
                            }
                        }
                    }
                }
            }
        }
    }
    let distinct = |items: Vec<String>| {
        let mut seen = HashSet::new();
        items
            .into_iter()
            .filter(|item| !item.is_empty() && seen.insert(item.clone()))
            .collect()
    };
    Identity {
        terms: distinct(terms),
        classes: distinct(classes),
    }
}

fn singular_last_word(term: &str) -> String {
    let mut words: Vec<&str> = term.split_whitespace().collect();
    let Some(last) = words.pop() else {
        return String::new();
    };
    let singular = if let Some(stem) = last.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = last.strip_suffix("ses") {
        format!("{stem}s")
    } else if last.ends_with('s') && !last.ends_with("ss") {
        last[..last.len() - 1].to_owned()
    } else {
        last.to_owned()
    };
    words.push(&singular);
    words.join(" ")
}

fn plural_last_word(term: &str) -> String {
    let mut words: Vec<&str> = term.split_whitespace().collect();
    let Some(last) = words.pop() else {
        return String::new();
    };
    let plural = if last.ends_with('y')
        && !matches!(last.chars().rev().nth(1), Some('a' | 'e' | 'i' | 'o' | 'u'))
    {
        format!("{}ies", &last[..last.len() - 1])
    } else if last.ends_with("ss")
        || last.ends_with('x')
        || last.ends_with("ch")
        || last.ends_with("sh")
    {
        format!("{last}es")
    } else if last.ends_with('s') {
        last.to_owned()
    } else {
        format!("{last}s")
    };
    words.push(&plural);
    words.join(" ")
}

fn explicit(text: &str, terms: &[String]) -> Option<String> {
    let padded = format!(" {} ", normalize(text));
    let mut ordered = terms.to_vec();
    ordered.sort_by_key(|term| std::cmp::Reverse(term.len()));
    ordered
        .into_iter()
        .find(|term| padded.contains(&format!(" {term} ")))
}

fn classify(text: &str, term: &str) -> (&'static str, &'static str, String) {
    let mut sentences = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for index in 0..bytes.len() {
        if matches!(bytes[index], b'.' | b'!' | b'?')
            && bytes
                .get(index + 1)
                .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            let sentence = text[start..index + 1].trim();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            start = index + 1;
        }
    }
    if start < text.len() {
        sentences.push(text[start..].trim());
    }
    let matching: Vec<&str> = sentences
        .into_iter()
        .filter(|sentence| format!(" {} ", normalize(sentence)).contains(&format!(" {term} ")))
        .collect();
    let source = if matching.is_empty() {
        vec![text]
    } else {
        matching
    };
    let actionable: Vec<&str> = source
        .iter()
        .copied()
        .filter(|sentence| !no_action_sentence(sentence))
        .collect();
    if actionable.is_empty() {
        return ("no_action_required", "low", source.join(" "));
    }
    let excerpt = source.join(" ");
    let normalized = normalize(&actionable.join(" "));
    let words: Vec<&str> = normalized.split_whitespace().collect();
    let has_word = |prefix: &str| words.iter().any(|word| word.starts_with(prefix));
    let has_exact_word = |word: &str| words.contains(&word);
    let has_phrase = |phrase: &str| format!(" {normalized} ").contains(&format!(" {phrase} "));
    if has_word("contraindicat") {
        ("contraindicated", "high", excerpt)
    } else if has_exact_word("avoid")
        || ["should not be used", "not recommended", "do not use"]
            .iter()
            .any(|phrase| has_phrase(phrase))
    {
        ("avoid", "high", excerpt)
    } else if has_word("monitor")
        || has_word("adjust")
        || [
            "dose reduction",
            "reduce dose",
            "reduced dose",
            "closely observe",
        ]
        .iter()
        .any(|phrase| has_phrase(phrase))
    {
        ("monitor_or_adjust", "moderate", excerpt)
    } else if has_exact_word("may")
        || has_exact_word("can")
        || has_exact_word("risk")
        || has_word("caution")
        || has_word("potential")
    {
        ("possible_or_caution", "low", excerpt)
    } else {
        ("unclassified", "unknown", excerpt)
    }
}

fn no_action_sentence(sentence: &str) -> bool {
    let normalized = normalize(sentence);
    let has_phrase = |phrase: &str| format!(" {normalized} ").contains(&format!(" {phrase} "));
    let no_dose = ["dose", "dosing"].iter().any(|noun| {
        ["adjustment", "adjustments"].iter().any(|form| {
            ["needed", "required", "necessary"].iter().any(|ending| {
                has_phrase(&format!("no {noun} {form} {ending}"))
                    || has_phrase(&format!("no {noun} {form} is {ending}"))
                    || has_phrase(&format!("no {noun} {form} are {ending}"))
            })
        })
    });
    let no_clinical = ["meaningful", "relevant", "significant"]
        .iter()
        .any(|word| {
            ["change", "effect", "interaction"]
                .iter()
                .any(|noun| has_phrase(&format!("no clinically {word} {noun}")))
        });
    no_dose || no_clinical
}

fn curated_pair(evidence: &review_evidence::Model, first: &str, second: &str) -> bool {
    if evidence.match_status != "reviewed_pair" {
        return false;
    }
    let curated = |value: &str| {
        value
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let matches = |candidate: &str, existing: &str| {
        evidence
            .candidate_terms
            .iter()
            .any(|term| curated(candidate).contains(&curated(term)))
            && evidence
                .interacting_terms
                .iter()
                .any(|term| curated(existing).contains(&curated(term)))
    };
    matches(first, second) || matches(second, first)
}

pub(super) fn matches<'a>(
    first: &str,
    second: &str,
    evidence: &'a [review_evidence::Model],
) -> Vec<EvidenceMatch<'a>> {
    let first_identity = identity(first, evidence);
    let second_identity = identity(second, evidence);
    evidence
        .iter()
        .filter_map(|row| {
            if !owns(row, first) && !owns(row, second) {
                return None;
            }
            let primary_is_first = owns(row, first);
            if curated_pair(row, first, second) {
                let matched_term = row
                    .interacting_terms
                    .iter()
                    .find(|term| {
                        overlaps(&normalize(first), &normalize(term))
                            || overlaps(&normalize(second), &normalize(term))
                    })
                    .or_else(|| row.interacting_terms.first())?
                    .clone();
                return Some(EvidenceMatch {
                    evidence: row,
                    primary_is_first,
                    matched_term: matched_term.clone(),
                    match_type: "curated",
                    confidence: row.match_confidence.clone(),
                    instruction: "unclassified",
                    risk: row.risk_level.clone(),
                    excerpt: row.evidence_text.clone(),
                    reason: format!(
                        "A reviewed rule identifies {matched_term} as the interacting medicine."
                    ),
                });
            }
            for (owner, other, owner_first) in [
                (&first_identity, &second_identity, true),
                (&second_identity, &first_identity, false),
            ] {
                if !owns(row, if owner_first { first } else { second })
                    || owner.terms.is_empty()
                    || other.terms.is_empty()
                {
                    continue;
                }
                let ingredient = explicit(&row.evidence_text, &other.terms);
                let classes: Vec<String> = other
                    .classes
                    .iter()
                    .flat_map(|term| [term.clone(), plural_last_word(term)])
                    .collect();
                let (matched_term, match_type, confidence) = if let Some(term) = ingredient {
                    (term, "ingredient", "high")
                } else if let Some(term) = explicit(&row.evidence_text, &classes) {
                    (term, "class", "moderate")
                } else {
                    continue;
                };
                let (instruction, risk, excerpt) = classify(&row.evidence_text, &matched_term);
                if instruction == "no_action_required" {
                    continue;
                }
                return Some(EvidenceMatch {
                    evidence: row,
                    primary_is_first: owner_first,
                    matched_term: matched_term.clone(),
                    match_type,
                    confidence: confidence.to_owned(),
                    instruction,
                    risk: risk.to_owned(),
                    excerpt,
                    reason: format!("The label explicitly names the {match_type} {matched_term}."),
                });
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{classify, medication_term, normalize, plural_last_word, singular_last_word};

    #[test]
    fn instruction_boundaries_preserve_the_source_sentence() {
        assert_eq!(
            classify("Cancer risk is discussed with warfarin.", "warfarin").0,
            "possible_or_caution"
        );
        assert_eq!(
            classify("Warfarin is unavoidable in this case.", "warfarin").0,
            "unclassified"
        );
        assert_eq!(
            classify(
                "No clinically significant risk was excluded for warfarin.",
                "warfarin"
            )
            .0,
            "possible_or_caution"
        );
        assert_eq!(
            classify(
                "No dose adjustment because monitoring is required for warfarin.",
                "warfarin"
            )
            .0,
            "monitor_or_adjust"
        );
        assert_eq!(
            classify(
                "No dose adjustments are required for warfarin. Avoid warfarin in pregnancy.",
                "warfarin"
            )
            .0,
            "avoid"
        );
        assert_eq!(
            classify("Monitor warfarin carefully. Other information.", "warfarin").2,
            "Monitor warfarin carefully."
        );
    }

    #[test]
    fn terminology_normalization_preserves_medication_and_class_boundaries() {
        assert_eq!(normalize("Warfarin [brand] 5mg"), "warfarin 5mg");
        assert_eq!(medication_term("Warfarin 5mg tablets"), "warfarin");
        assert_eq!(singular_last_word("benzodiazepines"), "benzodiazepine");
        assert_eq!(plural_last_word("benzodiazepine"), "benzodiazepines");
        assert_eq!(
            singular_last_word("calcium channel blockers"),
            "calcium channel blocker"
        );
    }
}
