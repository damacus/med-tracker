mod assignment;
mod fields;
mod overview;
mod pause;
mod schedule;
mod schedule_controls;

use crate::household_i18n::Locale;
use std::collections::{BTreeMap, BTreeSet};

pub use assignment::render_assignment_form;
pub use overview::{TreatmentRow, render_treatment_overview};
pub use pause::{PauseHistoryPage, PauseHistoryRow, render_pause_form, render_pause_history};
pub use schedule::render_schedule_form;

pub type FormErrors = BTreeMap<String, Vec<String>>;

#[derive(Clone, Debug, Default)]
pub struct TreatmentDraft {
    pub fields: BTreeMap<String, String>,
}

impl TreatmentDraft {
    pub fn value(&self, name: &str) -> &str {
        self.fields
            .get(name)
            .map(String::as_str)
            .unwrap_or_default()
    }

    pub fn indices(&self, prefix: &str, initial: usize) -> Vec<usize> {
        let indices = self
            .fields
            .keys()
            .filter_map(|name| {
                name.strip_prefix(prefix)
                    .and_then(|suffix| suffix.split('_').next())
                    .and_then(|index| index.parse::<usize>().ok())
            })
            .collect::<BTreeSet<_>>();
        if indices.is_empty() {
            (0..initial).collect()
        } else {
            indices.into_iter().collect()
        }
    }
}

pub struct TreatmentFormPage {
    pub household_name: String,
    pub slug: String,
    pub person_id: String,
    pub person_name: String,
    pub csrf: String,
    pub locale: Locale,
    pub action: String,
    pub editing: bool,
    pub schedule: bool,
    pub medications: Vec<(String, String)>,
    pub dosages: Vec<(String, String)>,
    pub units: Vec<(String, String)>,
    pub draft: TreatmentDraft,
    pub errors: FormErrors,
}
