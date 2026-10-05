use serde_yaml::Value;
use std::{collections::HashMap, fmt, sync::OnceLock};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Locale {
    En,
    Cy,
    Ga,
    Es,
    Pt,
}

impl Locale {
    pub const ALL: [Self; 5] = [Self::En, Self::Cy, Self::Ga, Self::Es, Self::Pt];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Cy => "cy",
            Self::Ga => "ga",
            Self::Es => "es",
            Self::Pt => "pt",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|locale| locale.as_str().eq_ignore_ascii_case(value.trim()))
    }

    pub fn resolve(cookie_locale: Option<&str>, accept_language: Option<&str>) -> Self {
        if let Some(locale) = cookie_locale.and_then(Self::parse) {
            return locale;
        }
        let mut selected = Self::En;
        let mut best_quality = 0.0_f32;
        for range in accept_language.unwrap_or_default().split(',') {
            let mut parts = range.trim().split(';');
            let language = parts.next().unwrap_or_default().trim();
            let locale = if language == "*" {
                Some(Self::En)
            } else {
                Self::parse(language.split('-').next().unwrap_or_default())
            };
            let Some(locale) = locale else { continue };
            let quality = match parts.next() {
                None => Some(1.0),
                Some(parameter) => parameter
                    .trim()
                    .strip_prefix("q=")
                    .and_then(|value| value.parse::<f32>().ok()),
            };
            let Some(quality) = quality else { continue };
            if parts.next().is_none()
                && quality.is_finite()
                && quality <= 1.0
                && quality > best_quality
            {
                best_quality = quality;
                selected = locale;
            }
        }
        selected
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TranslationError {
    Catalogue(String),
    MissingKey(String),
    UnsupportedKey(String),
    InvalidValue(String),
    MissingArgument(String),
}

impl fmt::Display for TranslationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalogue(message) => {
                write!(formatter, "Invalid translation catalogue: {message}")
            }
            Self::MissingKey(key) => write!(formatter, "Missing translation: {key}"),
            Self::UnsupportedKey(key) => {
                write!(formatter, "Unsupported household translation: {key}")
            }
            Self::InvalidValue(key) => write!(formatter, "Translation is not text: {key}"),
            Self::MissingArgument(name) => {
                write!(formatter, "Missing translation argument: {name}")
            }
        }
    }
}

impl std::error::Error for TranslationError {}

#[derive(Clone, Copy, Debug)]
pub struct Text {
    locale: Locale,
}

impl Text {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub fn source_locale(self, key: &str) -> Result<Locale, TranslationError> {
        self.value(key).map(|(_, locale)| locale)
    }

    pub fn api_error(self, message: &str) -> Option<String> {
        let (key, arguments): (&str, &[(&str, &str)]) = match message {
            "can't be blank" => ("errors.messages.blank", &[]),
            "is invalid" => ("errors.messages.invalid", &[]),
            "has already been taken" => ("errors.messages.taken", &[]),
            "is not a number" => ("errors.messages.not_a_number", &[]),
            "is not included in the list" => ("errors.messages.inclusion", &[]),
            "must be greater than 0" => ("errors.messages.greater_than", &[("count", "0")]),
            "must be greater than or equal to 0" => (
                "errors.messages.greater_than_or_equal_to",
                &[("count", "0")],
            ),
            _ => return None,
        };
        self.get(key, arguments).ok()
    }

    pub fn form_error(self, message: &str) -> Result<String, TranslationError> {
        if let Some(translated) = self.api_error(message) {
            return Ok(translated);
        }
        for key in [
            "stock_removals.errors.invalid_quantity",
            "stock_removals.errors.invalid_reason",
            "stock_removals.errors.invalid_note",
            "stock_removals.errors.invalid_submission",
            "stock_removals.errors.invalid_source",
            "stock_removals.errors.untracked",
            "stock_removals.errors.insufficient_stock",
            "stock_removals.errors.changed_submission",
            "medications.stock.original_token",
            "medications.stock.option_readonly",
            "medications.stock.invalid_quantity",
        ] {
            if self.get(key, &[]).as_deref() == Ok(message) {
                return Ok(message.to_owned());
            }
        }
        for known in [
            "can't be blank",
            "is invalid",
            "has already been taken",
            "is not a number",
            "is not included in the list",
            "must be greater than 0",
            "must be greater than or equal to 0",
        ] {
            if self.api_error(known).as_deref() == Some(message) {
                return Ok(message.to_owned());
            }
        }
        self.get("errors.messages.form_invalid", &[])
    }

    pub fn get(self, key: &str, arguments: &[(&str, &str)]) -> Result<String, TranslationError> {
        let (value, _) = self.value(key)?;
        let template = value
            .as_str()
            .ok_or_else(|| TranslationError::InvalidValue(key.to_owned()))?;
        interpolate(template, arguments)
    }

    pub fn plural(
        self,
        key: &str,
        count: u64,
        arguments: &[(&str, &str)],
    ) -> Result<String, TranslationError> {
        let (value, _) = self.value(key)?;
        let branch = if count == 0 && value.get("zero").is_some() {
            "zero"
        } else if count == 1 {
            "one"
        } else {
            "other"
        };
        let template = value
            .get(branch)
            .and_then(Value::as_str)
            .ok_or_else(|| TranslationError::InvalidValue(key.to_owned()))?;
        let count = count.to_string();
        let mut values: Vec<_> = arguments
            .iter()
            .copied()
            .filter(|(name, _)| *name != "count")
            .collect();
        values.push(("count", &count));
        interpolate(template, &values)
    }

    fn value(self, key: &str) -> Result<(&'static Value, Locale), TranslationError> {
        if !allowed(key) {
            return Err(TranslationError::UnsupportedKey(key.to_owned()));
        }
        let catalogues = catalogues()?;
        if let Some(value) = lookup(&catalogues[&self.locale], key) {
            return Ok((value, self.locale));
        }
        lookup(&catalogues[&Locale::En], key)
            .map(|value| (value, Locale::En))
            .ok_or_else(|| TranslationError::MissingKey(key.to_owned()))
    }
}

fn allowed(key: &str) -> bool {
    [
        "admin.households.",
        "admin.nhs_dmd_imports.",
        "locations.",
        "forms.locations.",
        "people.",
        "forms.people.",
        "schedules.",
        "forms.medications.",
        "medications.",
        "dosages.",
        "person_medications.",
        "treatments.",
        "medication_pauses.",
        "dose_outcomes.",
        "reports.",
        "stock_removals.",
        "layouts.",
        "errors.messages.",
        "activerecord.attributes.",
        "dashboard.",
    ]
    .iter()
    .any(|prefix| key.starts_with(prefix))
}

fn lookup<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    key.split('.').try_fold(value, |node, part| node.get(part))
}

fn catalogues() -> Result<&'static HashMap<Locale, Value>, TranslationError> {
    static CATALOGUES: OnceLock<Result<HashMap<Locale, Value>, String>> = OnceLock::new();
    CATALOGUES
        .get_or_init(|| {
            [
                (
                    Locale::En,
                    include_str!("../../../rails/config/locales/en.yml"),
                ),
                (
                    Locale::Cy,
                    include_str!("../../../rails/config/locales/cy.yml"),
                ),
                (
                    Locale::Ga,
                    include_str!("../../../rails/config/locales/ga.yml"),
                ),
                (
                    Locale::Es,
                    include_str!("../../../rails/config/locales/es.yml"),
                ),
                (
                    Locale::Pt,
                    include_str!("../../../rails/config/locales/pt.yml"),
                ),
            ]
            .into_iter()
            .map(|(locale, source)| {
                let parsed: Value =
                    serde_yaml::from_str(source).map_err(|error| error.to_string())?;
                let root = parsed
                    .get(locale.as_str())
                    .filter(|value| value.is_mapping())
                    .ok_or_else(|| format!("{} wrapper is missing", locale.as_str()))?;
                Ok((locale, root.clone()))
            })
            .collect()
        })
        .as_ref()
        .map_err(|message| TranslationError::Catalogue(message.clone()))
}

fn interpolate(template: &str, arguments: &[(&str, &str)]) -> Result<String, TranslationError> {
    let mut output = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find("%{") {
        if start > 0 && remaining.as_bytes()[start - 1] == b'%' {
            output.push_str(&remaining[..start - 1]);
            output.push_str("%{");
            remaining = &remaining[start + 2..];
            continue;
        }
        output.push_str(&remaining[..start]);
        let variable = &remaining[start + 2..];
        let end = variable
            .find('}')
            .ok_or_else(|| TranslationError::InvalidValue(template.to_owned()))?;
        let name = &variable[..end];
        let value = arguments
            .iter()
            .find(|(argument, _)| *argument == name)
            .map(|(_, value)| *value)
            .ok_or_else(|| TranslationError::MissingArgument(name.to_owned()))?;
        output.push_str(value);
        remaining = &variable[end + 1..];
    }
    output.push_str(remaining);
    Ok(output)
}
