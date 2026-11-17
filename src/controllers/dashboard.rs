use super::medications::{authentication_error, begin, operation_error, request_id, unavailable};
use crate::controllers::shell::{ShellView as TeraView, ShellViewEngine as ViewEngine};
use crate::models::{
    access,
    care::{
        administration, browser_query, dashboard, dose_occurrences, doses, report_pdf, reports,
    },
    entities::{account, location, medication, person},
    errors::OperationError,
};
use axum::{
    Extension,
    extract::{Form, Query},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use chrono::Timelike;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Default, Deserialize)]
struct Selection {
    dashboard_person_id: Option<String>,
    dashboard_grouping: Option<String>,
    record_medication_id: Option<i64>,
    record_source_type: Option<String>,
    record_source_id: Option<i64>,
    record_person_id: Option<i64>,
    record_occurrence_key: Option<String>,
    record_occurrence_etag: Option<String>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/dashboard", get(show))
        .add("/{slug}/dashboard/doses/{id}", post(take))
}

async fn show(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    (Query(query), failure): (Query<Selection>, Option<Extension<DoseFailure>>),
) -> Response {
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let owner = match account::Entity::find_by_id(principal.account_id())
        .one(tenant.transaction())
        .await
    {
        Ok(Some(value)) => value,
        _ => return unavailable(),
    };
    let selected = match query.dashboard_person_id.as_deref() {
        None => {
            let candidates = match access::person_queries::household_people(tenant.scope())
                .filter(
                    person::Column::Id
                        .in_subquery(access::participating_people(tenant.membership())),
                )
                .order_by_asc(person::Column::Name)
                .order_by_asc(person::Column::Id)
                .all(tenant.transaction())
                .await
            {
                Ok(value) => value,
                Err(_) => return unavailable(),
            };
            candidates
                .iter()
                .find(|person| person.id == tenant.membership().person_id)
                .map(|person| person.id)
        }
        Some("all") => None,
        Some(value) => match value.parse::<i64>() {
            Ok(id) if id > 0 => Some(id),
            _ => return operation_error(OperationError::NotFound),
        },
    };
    let grouping = if query.dashboard_grouping.as_deref() == Some("time") {
        "time"
    } else {
        "person"
    };
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = match report_pdf::translations(locale) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let snapshot =
        match dashboard::read(&tenant, principal.time_zone(), selected, &slug, &labels).await {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
    let own_person = match person::Entity::find_by_id(tenant.membership().person_id)
        .one(tenant.transaction())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let name = own_person
        .as_ref()
        .map_or(owner.email.as_str(), |person| person.name.as_str());
    let can_manage = match administration::can_manage(&tenant).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let can_add_medication = match crate::models::care::medications::crud::can_create(&tenant).await
    {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let Ok(authenticity_token) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = labels["dashboard"]["title"].clone();
    data["lang"] = json!(locale);
    data["labels"] = labels.clone();
    data["slug"] = json!(slug);
    data["name"] = json!(name);
    data["initials"] = json!(
        name.split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .flat_map(char::to_uppercase)
            .collect::<String>()
    );
    let own_avatar = own_person.as_ref().and_then(|person| {
        snapshot["people"]
            .as_array()?
            .iter()
            .find(|option| option["id"] == person.id)
    });
    data["avatar_attached"] = json!(own_avatar.is_some_and(|option| option["avatar"] == true));
    data["gravatar_url"] = json!(own_avatar.and_then(|option| option["gravatar_url"].as_str()));
    data["authenticity_token"] = json!(authenticity_token);
    data["role_label"] =
        labels["admin"]["labels"]["membership_roles"][&tenant.membership().role].clone();
    data["sidebar_links"] = json!(navigation_links(&slug, &labels, can_manage));
    data["can_add_medication"] = json!(can_add_medication);
    let status = if let Some(Extension(failure)) = &failure {
        super::medications::forms::status(&failure.error)
    } else {
        StatusCode::OK
    };
    if let Some(medication_id) = query.record_medication_id {
        let mut detail =
            match browser_query::detail(&tenant, &medication_id.to_string(), principal.time_zone())
                .await
            {
                Ok(detail) => detail,
                Err(error) => return operation_error(error),
            };
        let mut draft = if let Some(Extension(failure)) = &failure {
            failure.draft.clone()
        } else {
            let Some(source) = detail.assignments.iter().find(|source| {
                source.can_record
                    && Some(source.source_type) == query.record_source_type.as_deref()
                    && Some(source.id) == query.record_source_id
                    && Some(source.person_id) == query.record_person_id
            }) else {
                return operation_error(OperationError::NotFound);
            };
            let mut draft =
                super::medications::forms::dose_for(source, medication_id, principal.time_zone());
            if let Some(key) = query
                .record_occurrence_key
                .as_deref()
                .filter(|key| !key.is_empty())
            {
                let today = chrono::Utc::now()
                    .with_timezone(&principal.time_zone())
                    .date_naive();
                let occurrences = match dose_occurrences::with_dashboard_timezone(
                    principal.time_zone(),
                    dose_occurrences::projected_for_dashboard(&tenant, &[source.person_id], today),
                )
                .await
                {
                    Ok(rows) => rows,
                    Err(error) => return operation_error(error),
                };
                let Some(occurrence) = occurrences.iter().find(|row| {
                    row.occurrence_key == key
                        && row.source_type == source.source_type
                        && row.source_id == source.id
                        && row.person_id == source.person_id
                        && row.medication_id == medication_id
                }) else {
                    return operation_error(OperationError::NotFound);
                };
                if occurrence.outcome != "open"
                    || query
                        .record_occurrence_etag
                        .as_deref()
                        .filter(|value| !value.is_empty())
                        != occurrence.occurrence_etag.as_deref()
                {
                    return operation_error(OperationError::Conflict {
                        code: "sync_conflict".into(),
                        details: json!({"error":"This scheduled dose has changed. Refresh the timeline to review its current outcome."}),
                    });
                }
                draft.insert("record_occurrence_key".into(), key.to_owned());
                if let Some(etag) = query.record_occurrence_etag.as_ref() {
                    draft.insert("record_occurrence_etag".into(), etag.clone());
                }
            } else if !snapshot["rows"].as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["source_type"] == source.source_type
                        && row["source_id"] == source.id
                        && row["person_id"] == source.person_id
                        && row["medication_id"] == medication_id
                        && row["as_needed"] == true
                })
            }) {
                return operation_error(OperationError::NotFound);
            }
            draft
        };
        detail.assignments.retain(|source| {
            source.source_type == super::medications::forms::field(&draft, "source_type")
                && source.id.to_string() == super::medications::forms::field(&draft, "source_id")
        });
        if failure.is_some()
            && let Ok(preview) = doses::browser_preview(
                &tenant,
                medication_id,
                super::medications::forms::field(&draft, "source_type"),
                super::medications::forms::field(&draft, "source_id"),
                super::medications::forms::field(&draft, "taken_at"),
                principal.time_zone(),
            )
            .await
            && let (Some(amount), Some(unit)) = (preview.amount, preview.unit)
            && let Some(source) = detail.assignments.first_mut()
        {
            source.amount = amount;
            source.unit = unit;
            source.available = preview.available;
        }
        let source_row = snapshot["rows"].as_array().and_then(|rows| {
            rows.iter().find(|row| {
                row["source_type"].as_str()
                    == Some(super::medications::forms::field(&draft, "source_type"))
                    && row["source_id"]
                        .as_i64()
                        .map(|id| id.to_string())
                        .as_deref()
                        == Some(super::medications::forms::field(&draft, "source_id"))
                    && (failure.is_some()
                        || row["as_needed"] == true
                        || row["occurrence_key"].as_str()
                            == Some(super::medications::forms::field(
                                &draft,
                                "record_occurrence_key",
                            )))
            })
        });
        let stock_ids: Vec<i64> = source_row
            .and_then(|row| row["stock_ids"].as_array())
            .map(|ids| ids.iter().filter_map(serde_json::Value::as_i64).collect())
            .unwrap_or_default();
        let stock_options = match stock_options(&tenant, &stock_ids).await {
            Ok(options) => options,
            Err(error) => return operation_error(error),
        };
        let selected_stock = super::medications::forms::field(&draft, "taken_from_medication_id")
            .parse::<i64>()
            .ok();
        if !selected_stock.is_some_and(|id| stock_ids.contains(&id))
            && let Some(id) = stock_ids.first()
        {
            draft.insert("taken_from_medication_id".into(), id.to_string());
        }
        let mut dose = match super::medications::rendering::dose_context(
            &slug,
            &detail,
            super::medications::rendering::DosePresentation {
                zone: principal.time_zone(),
                language: locale,
                draft: &draft,
                error: failure.as_ref().map(|Extension(failure)| &failure.error),
                open: true,
                navigation: None,
            },
        ) {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
        dose["authenticity_token"] = json!(authenticity_token);
        dose["can_record_any"] = json!(detail.assignments.iter().any(|source| source.can_record));
        dose["dashboard_return"] = json!(true);
        dose["record_occurrence_key"] = json!(super::medications::forms::field(
            &draft,
            "record_occurrence_key"
        ));
        dose["record_occurrence_etag"] = json!(super::medications::forms::field(
            &draft,
            "record_occurrence_etag"
        ));
        if let Some(Extension(DoseFailure {
            error: OperationError::Validation { details },
            ..
        })) = &failure
            && details["errors"]["taken_at"] == json!(["does not match the occurrence window"])
        {
            dose["historical_record_url"] =
                json!(format!("/households/{slug}/medications/{medication_id}"));
        }
        dose["stock_options"] = json!(stock_options);
        dose["dose_i18n"]["stock_source"] =
            labels["medications"]["take_action"]["stock_source"].clone();
        dose["as_needed"] = source_row.map_or(json!(false), |row| row["as_needed"].clone());
        dose["dashboard_person_id"] =
            json!(selected.map_or_else(|| "all".to_owned(), |id| id.to_string()));
        dose["dashboard_grouping"] = json!(grouping);
        data["dose"] = dose;
    }
    {
        let people = match reports::accessible_people(&tenant, access::PersonAccess::View).await {
            Ok(people) => people
                .into_iter()
                .filter(|person| selected.is_none_or(|id| person.id == id))
                .collect::<Vec<_>>(),
            Err(error) => return operation_error(error),
        };
        let now = chrono::Utc::now();
        let today = now.with_timezone(&principal.time_zone()).date_naive();
        let history = match reports::ordinary_history(
            &tenant,
            &people,
            today - chrono::Duration::days(6),
            today,
            now,
            principal.time_zone(),
        )
        .await
        {
            Ok(history) => history,
            Err(error) => return operation_error(error),
        };
        data["insights"] = json!(history.smart_insights);
        data["insights_start_date"] = json!(history.start_date.to_string());
        data["insights_end_date"] = json!(history.end_date.to_string());
    }
    data["dashboard"] = snapshot;
    data["grouping"] = json!(grouping);
    data["version"] = json!(env!("CARGO_PKG_VERSION"));
    let hour = chrono::Utc::now()
        .with_timezone(&principal.time_zone())
        .hour();
    let greeting = match hour {
        5..=11 => "greeting_morning",
        12..=17 => "greeting_afternoon",
        _ => "greeting_evening",
    };
    data["greeting"] = json!(
        labels["dashboard"][greeting]
            .as_str()
            .unwrap_or("")
            .replace("%{name}", name.split_whitespace().next().unwrap_or(name))
    );
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    match format::render().view(&view, "dashboard/show.html", data) {
        Ok(response) => (
            status,
            token,
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

async fn stock_options(
    tenant: &access::TenantTransaction,
    ids: &[i64],
) -> Result<Vec<serde_json::Value>, OperationError> {
    let medicines = access::medication_scope(tenant)
        .filter(medication::Column::Id.is_in(ids.iter().copied()))
        .all(tenant.transaction())
        .await?;
    let locations: HashMap<_, _> = location::Entity::find()
        .filter(location::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(location::Column::Id.is_in(medicines.iter().map(|medicine| medicine.location_id)))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|location| (location.id, location.name))
        .collect();
    Ok(ids
        .iter()
        .filter_map(|id| {
            let medicine = medicines.iter().find(|medicine| medicine.id == *id)?;
            let location = locations.get(&medicine.location_id)?;
            let supply = medicine
                .current_supply
                .map(|value| value.normalize().to_string())
                .unwrap_or_else(|| "Untracked".into());
            let unit = if medicine.dose_unit.as_deref() == Some("ml") {
                "ml"
            } else if medicine.current_supply == Some(sea_orm::prelude::Decimal::ONE) {
                "unit"
            } else {
                "units"
            };
            Some(json!({"id":id.to_string(),"label":format!("{location} · {supply} {unit}")}))
        })
        .collect())
}

fn navigation_links(
    slug: &str,
    labels: &serde_json::Value,
    can_manage: bool,
) -> Vec<serde_json::Value> {
    [
        ("dashboard", "dashboard"),
        ("inventory", "medications"),
        ("locations", "locations"),
        ("people", "people"),
        ("finder", "medications/finder"),
        ("medicine_reviews", "medicine-reviews"),
        ("reports", "reports"),
        ("profile", "profile"),
        ("administration", "admin"),
    ]
    .into_iter()
    .filter(|(id, _)| *id != "administration" || can_manage)
    .map(|(id, path)| {
        let label = match id {
            "profile" => &labels["layouts"]["profile_menu"]["profile"],
            "dashboard" => &labels["layouts"]["mobile_rail"]["home"],
            "finder" => &labels["layouts"]["mobile_rail"]["finder"],
            _ => &labels["layouts"]["sidebar"][id],
        };
        json!({"id":id,"label":label,"href":format!("/households/{slug}/{path}")})
    })
    .collect()
}

#[derive(Clone)]
struct DoseFailure {
    draft: HashMap<String, String>,
    error: OperationError,
}

async fn take(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, i64)>,
    session: Session<SessionPgPool>,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(draft): Form<HashMap<String, String>>,
) -> Response {
    let forms = &draft;
    if token
        .verify(super::medications::forms::field(
            forms,
            "authenticity_token",
        ))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request.clone())).await
    {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        let detail = browser_query::detail(&tenant, &id.to_string(), principal.time_zone()).await?;
        let input = super::medications::forms::take(&detail, &draft, principal.time_zone())?;
        let kind = super::medications::forms::field(&draft, "source_type");
        let source_id = super::medications::forms::field(&draft, "source_id");
        let key = super::medications::forms::optional(&draft, "record_occurrence_key");
        let secret = dose_occurrences::configured_signing_key(ctx.config.settings.as_ref())?;
        let as_needed = dose_occurrences::with_dashboard_timezone(
            principal.time_zone(),
            dose_occurrences::browser_take_target(
                &tenant,
                kind,
                source_id,
                id,
                key.as_deref(),
                &secret,
            ),
        )
        .await?;
        if as_needed {
            doses::execute_attempt_in_timezone(
                &tenant,
                input,
                principal.time_zone(),
                Some(principal.provenance()),
            )
            .await?;
        } else {
            dose_occurrences::with_dashboard_timezone(
                principal.time_zone(),
                dose_occurrences::change(
                    &tenant,
                    (kind, source_id),
                    "take",
                    dose_occurrences::ChangeInput::browser_take(
                        key.as_deref().unwrap_or_default(),
                        input,
                    )?,
                    super::medications::forms::optional(&draft, "record_occurrence_etag")
                        .as_deref(),
                    &secret,
                    Some(principal.provenance()),
                ),
            )
            .await?;
        }
        Ok::<(), OperationError>(())
    }
    .await;
    let query = Selection {
        dashboard_person_id: draft.get("dashboard_person_id").cloned(),
        dashboard_grouping: draft.get("dashboard_grouping").cloned(),
        record_medication_id: Some(id),
        ..Selection::default()
    };
    match result {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            let person = query
                .dashboard_person_id
                .as_deref()
                .filter(|value| *value == "all" || value.parse::<i64>().is_ok())
                .unwrap_or("all");
            let grouping = if query.dashboard_grouping.as_deref() == Some("time") {
                "time"
            } else {
                "person"
            };
            axum::response::Redirect::to(&format!("/households/{slug}/dashboard?dashboard_person_id={person}&dashboard_grouping={grouping}")).into_response()
        }
        Err(mut error) => {
            if let OperationError::Validation { details } = &mut error {
                if details["errors"]["taken_at"] == json!(["does not match the occurrence window"])
                {
                    details["error"] = json!(
                        "This time is outside the selected scheduled dose window. Record a past dose from medication details."
                    );
                } else if details["code"] == "invalid_occurrence" {
                    details["error"] = json!(
                        "This scheduled dose is unavailable. Refresh today's timeline and choose a dose that is due."
                    );
                } else if details["errors"]["key"].is_array() {
                    details["error"] = json!("Choose the scheduled dose from today's timeline.");
                }
            }
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            show(
                State(ctx),
                Path(slug),
                session,
                (headers, request),
                token,
                ViewEngine(view),
                (Query(query), Some(Extension(DoseFailure { draft, error }))),
            )
            .await
        }
    }
}
