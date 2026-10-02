use super::*;
use medtracker_web::household::path_segment;
use medtracker_web::treatments::{TreatmentDraft, TreatmentFormPage};
use std::collections::BTreeMap;
use uuid::Uuid;

mod context;
mod forms;
mod intentions;
mod overview;
mod pause;
mod payload;
mod schedule_config;
mod writing;

use context::{load, source, Context};
pub(super) use overview::rows;

#[derive(Clone, Copy)]
enum Kind {
    Assignment,
    Schedule,
}

impl Kind {
    fn resource(self) -> &'static str {
        match self {
            Self::Assignment => "person_medications",
            Self::Schedule => "schedules",
        }
    }

    fn body_key(self) -> &'static str {
        match self {
            Self::Assignment => "person_medication",
            Self::Schedule => "schedule",
        }
    }

    fn browser_resource(self) -> &'static str {
        match self {
            Self::Assignment => "assignments",
            Self::Schedule => "schedules",
        }
    }
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/households/{slug}/people/{id}/assignments/new",
            get(forms::new_assignment),
        )
        .route(
            "/households/{slug}/people/{id}/assignments",
            post(writing::create_assignment),
        )
        .route(
            "/households/{slug}/people/{id}/assignments/{source}/edit",
            get(forms::edit_assignment),
        )
        .route(
            "/households/{slug}/people/{id}/assignments/{source}",
            post(writing::update_assignment),
        )
        .route(
            "/households/{slug}/people/{id}/schedules/new",
            get(forms::new_schedule),
        )
        .route(
            "/households/{slug}/people/{id}/schedules",
            post(writing::create_schedule),
        )
        .route(
            "/households/{slug}/people/{id}/schedules/{source}/edit",
            get(forms::edit_schedule),
        )
        .route(
            "/households/{slug}/people/{id}/schedules/{source}",
            post(writing::update_schedule),
        )
        .route(
            "/households/{slug}/people/{id}/{resource}/{source}/{action}",
            get(pause::show).post(pause::save),
        )
}

fn base(slug: &str, person: &str, kind: Kind) -> String {
    format!(
        "/households/{}/people/{}/{}",
        path_segment(slug),
        path_segment(person),
        kind.browser_resource()
    )
}

type Errors = BTreeMap<String, Vec<String>>;

struct FormLocation {
    slug: String,
    person: String,
    id: Option<String>,
    kind: Kind,
}
