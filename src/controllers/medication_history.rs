use crate::models::care::{
    doses::DoseSource,
    take_history::{HistoricalActor, self_history},
};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    pub(crate) after: Option<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) limit: Option<u32>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Actor {
    Known {
        person_id: String,
        membership_id: Option<String>,
        name: String,
    },
    Unknown,
    Imported,
}

impl From<HistoricalActor> for Actor {
    fn from(actor: HistoricalActor) -> Self {
        match actor {
            HistoricalActor::Known {
                person_id,
                membership_id,
                name,
            } => Self::Known {
                person_id: person_id.to_string(),
                membership_id: membership_id.map(|id| id.to_string()),
                name,
            },
            HistoricalActor::Unknown => Self::Unknown,
            HistoricalActor::Imported => Self::Imported,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Source {
    Schedule { id: String, portable_id: String },
    Assignment { id: String, portable_id: String },
}

#[derive(Serialize)]
struct Stock {
    medication_id: String,
    portable_id: Option<String>,
    name: Option<String>,
}

#[derive(Serialize)]
struct Location {
    id: String,
    portable_id: Option<String>,
    name: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct Entry {
    history_entry_id: String,
    id: String,
    portable_id: String,
    client_uuid: Option<String>,
    person_id: String,
    person_portable_id: String,
    origin_household_id: String,
    subject_name: String,
    medication_id: String,
    medication_portable_id: String,
    medication_name: Option<String>,
    administration: &'static str,
    dose_amount: Option<String>,
    dose_unit: Option<String>,
    taken_at: Option<String>,
    recorded_at: String,
    updated_at: String,
    source: Source,
    selected_stock: Option<Stock>,
    location: Option<Location>,
    actor: Actor,
}

impl From<self_history::Entry> for Entry {
    fn from(entry: self_history::Entry) -> Self {
        let take = entry.take;
        let source = match take.source {
            DoseSource::Schedule(id) => Source::Schedule {
                id: id.to_string(),
                portable_id: take.source_portable_id,
            },
            DoseSource::Assignment(id) => Source::Assignment {
                id: id.to_string(),
                portable_id: take.source_portable_id,
            },
        };
        Self {
            history_entry_id: entry.history_entry_id.to_string(),
            id: take.id.to_string(),
            portable_id: take.portable_id,
            client_uuid: take.client_uuid,
            person_id: take.person_id.to_string(),
            person_portable_id: take.person_portable_id,
            origin_household_id: take.household_id.to_string(),
            subject_name: take.subject_name,
            medication_id: take.medication_id.to_string(),
            medication_portable_id: take.medication_portable_id,
            medication_name: take.medication_name,
            administration: take.administration.label(),
            dose_amount: take.dose_amount.map(|amount| amount.to_string()),
            dose_unit: take.dose_unit,
            taken_at: take.taken_at.map(precise_timestamp),
            recorded_at: precise_timestamp(entry.recorded_at),
            updated_at: precise_timestamp(take.updated_at),
            source,
            selected_stock: take.taken_from_medication_id.map(|id| Stock {
                medication_id: id.to_string(),
                portable_id: take.taken_from_medication_portable_id,
                name: entry.stock_name,
            }),
            location: take.taken_from_location_id.map(|id| Location {
                id: id.to_string(),
                portable_id: take.taken_from_location_portable_id,
                name: take.location_name,
            }),
            actor: take.actor.into(),
        }
    }
}

fn precise_timestamp(time: chrono::NaiveDateTime) -> String {
    time.and_utc()
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}
