use super::*;
use serde::{Deserialize, Deserializer};

#[derive(Default)]
pub(crate) enum UpdateField<T> {
    #[default]
    Missing,
    Value(T),
    Invalid,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for UpdateField<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Decoded<T> {
            Value(T),
            Invalid(serde::de::IgnoredAny),
        }
        Ok(match Decoded::deserialize(deserializer)? {
            Decoded::Value(value) => Self::Value(value),
            Decoded::Invalid(_) => Self::Invalid,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PersonKind {
    Adult,
    Minor,
    DependentAdult,
}

impl PersonKind {
    pub(crate) fn code(self) -> i32 {
        match self {
            Self::Adult => 0,
            Self::Minor => 1,
            Self::DependentAdult => 2,
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PersonUpdateInput {
    pub(crate) name: UpdateField<String>,
    pub(crate) email: UpdateField<Option<String>>,
    pub(crate) date_of_birth: UpdateField<NaiveDate>,
    pub(crate) person_type: UpdateField<PersonKind>,
    pub(crate) has_capacity: UpdateField<bool>,
    #[serde(skip)]
    pub(crate) invalid_shape: bool,
}

impl PersonUpdateInput {
    pub(crate) fn from_fields(fields: &serde_json::Map<String, Value>) -> Self {
        Self::deserialize(
            serde::de::value::MapDeserializer::<_, serde_json::Error>::new(
                fields.iter().map(|(key, value)| (key.as_str(), value)),
            ),
        )
        .unwrap_or_else(|_| Self {
            invalid_shape: true,
            ..Self::default()
        })
    }

    pub(super) fn valid_shape(&self) -> bool {
        !self.invalid_shape
            && !(matches!(self.name, UpdateField::Missing)
                && matches!(self.email, UpdateField::Missing)
                && matches!(self.date_of_birth, UpdateField::Missing)
                && matches!(self.person_type, UpdateField::Missing)
                && matches!(self.has_capacity, UpdateField::Missing))
    }
}
