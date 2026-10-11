use super::*;
use crate::models::{
    access,
    care::{assignments, dosages, medication_lookup, treatments},
    entities::{account, dosage, person},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};

pub(super) async fn open(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    token: &CsrfToken,
    view: &TeraView,
    request: WizardRequest<'_>,
) -> Response {
    let WizardRequest {
        slug,
        request_id,
        mut draft,
        error,
    } = request;
    let field_errors = if matches!(error, Some(OperationError::Validation { details }) if details["errors"].is_object())
    {
        management::errors(error)
            .into_iter()
            .filter(|entry| {
                matches!(
                    entry["field"].as_str(),
                    Some(
                        "name"
                            | "friendly_name"
                            | "description"
                            | "location_id"
                            | "dose_amount"
                            | "dose_unit"
                            | "schedule_type"
                            | "current_supply"
                            | "reorder_threshold"
                            | "warnings"
                    )
                )
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if let Some(field) = field_errors
        .first()
        .and_then(|entry| entry["field"].as_str())
    {
        let step = match field {
            "name" | "friendly_name" | "description" | "location_id" => Some(0),
            "dose_amount" | "dose_unit" | "frequency" | "schedule_type" => Some(1),
            "current_supply" | "reorder_threshold" => Some(2),
            "warnings" => Some(3),
            _ => None,
        };
        if let Some(step) = step {
            draft.insert("wizard_step".into(), step.to_string());
        }
    }
    let (principal, tenant) = match begin(ctx, session, slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    match medications::crud::can_create(&tenant).await {
        Ok(true) => {}
        Ok(false) => return operation_error(OperationError::Forbidden),
        Err(error) => return operation_error(error),
    }
    let context = match browser_query::form_context(&tenant, None).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let account = match account::Entity::find_by_id(principal.account_id())
        .one(tenant.transaction())
        .await
    {
        Ok(Some(value)) => value,
        _ => return unavailable(),
    };
    let wizard_variant = match account.preferences["wizard_variant"].as_str() {
        Some("modal") => "modal",
        Some("slideover") => "slideover",
        _ => "fullpage",
    };
    let duplicate_choices = if error.is_some() && !forms::field(&draft, "name").is_empty() {
        let candidate = json!({"barcode":forms::field(&draft,"barcode"),"code":forms::field(&draft,"dmd_code"),"name":forms::field(&draft,"name")});
        match medication_lookup::stock_matches(&tenant, &candidate).await {
            Ok(rows) => rows.into_iter().map(|row| json!({"name":row.friendly_name.as_deref().filter(|name|!name.is_empty()).or(row.name.as_deref()).unwrap_or("Medication"),"location":context.locations.iter().find(|location|location.id==row.location_id).map(|location|location.name.as_str()).unwrap_or("Location"),"path":format!("/households/{slug}/medications/{}?refill=true",row.id)})).collect::<Vec<_>>(),
            Err(error) => return operation_error(error),
        }
    } else {
        Vec::new()
    };
    let people = match person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person::Column::Id.in_subquery(access::granted_people(tenant.membership())))
        .order_by_asc(person::Column::Name)
        .all(tenant.transaction())
        .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let today = chrono::Utc::now()
        .with_timezone(&principal.time_zone())
        .date_naive();
    let default_system = if draft
        .get("dmd_code")
        .is_some_and(|code| !code.trim().is_empty())
    {
        "https://dmd.nhs.uk"
    } else {
        ""
    };
    for (key, value) in [
        ("name", ""),
        ("friendly_name", ""),
        ("description", ""),
        ("warnings", ""),
        ("barcode", ""),
        ("dmd_code", ""),
        ("dmd_system", default_system),
        ("dmd_concept_class", ""),
        ("category", ""),
        ("dose_amount", ""),
        ("dose_unit", "tablet"),
        ("current_supply", ""),
        ("reorder_threshold", "0"),
        ("schedule_type", "multiple_daily"),
        ("times", "08:00, 20:00"),
        ("frequency", "Twice daily"),
        ("max_daily_doses", "2"),
        ("min_hours_between_doses", "6"),
        ("dates", ""),
        ("person_id", ""),
        ("step_count", "0"),
    ] {
        draft.entry(key.into()).or_insert(value.into());
    }
    draft
        .entry("start_date".into())
        .or_insert(today.to_string());
    draft
        .entry("end_date".into())
        .or_insert((today + chrono::Duration::days(365)).to_string());
    draft.entry("location_id".into()).or_insert_with(|| {
        context
            .locations
            .first()
            .map(|value| value.id.to_string())
            .unwrap_or_default()
    });
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = rendering::appearance_context();
    data["title"] = json!("Add a New Medication");
    data["slug"] = json!(slug);
    data["selected_location"] = json!(forms::field(&draft, "location_id").parse::<i64>().ok());
    data["selected_person"] = json!(forms::field(&draft, "person_id").parse::<i64>().ok());
    data["draft"] = json!(draft);
    data["duplicate_choices"] = json!(duplicate_choices);
    data["locations"] = json!(context.locations);
    data["people"] = json!(
        people
            .into_iter()
            .map(|person| json!({"id":person.id,"name":person.name}))
            .collect::<Vec<_>>()
    );
    data["authenticity_token"] = json!(authenticity);
    for field in [
        "name",
        "friendly_name",
        "description",
        "location_id",
        "dose_amount",
        "dose_unit",
        "schedule_type",
        "current_supply",
        "reorder_threshold",
        "warnings",
    ] {
        data[format!("invalid_{field}")] =
            json!(field_errors.iter().any(|entry| entry["field"] == field));
    }
    data["errors"] = json!(field_errors);
    data["error"] = json!(if data["errors"]
        .as_array()
        .is_some_and(|errors| !errors.is_empty())
    {
        None
    } else {
        error.map(forms::message)
    });
    data["wizard_variant"] = json!(wizard_variant);
    data["units"] = json!([
        "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet",
        "pad"
    ]);
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    match format::render().view(view, "medications/wizard.html", data) {
        Ok(response) => (
            error.map(forms::status).unwrap_or(StatusCode::OK),
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

pub(super) struct WizardRequest<'a> {
    pub slug: &'a str,
    pub request_id: String,
    pub draft: HashMap<String, String>,
    pub error: Option<&'a OperationError>,
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    if token
        .verify(forms::field(&draft, "authenticity_token"))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = save(&tenant, &principal, &draft).await;
    match result {
        Ok((id, name)) => {
            let options = match dosage::Entity::find()
                .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
                .filter(dosage::Column::MedicationId.eq(id))
                .order_by_asc(dosage::Column::Id)
                .all(tenant.transaction())
                .await
            {
                Ok(options) => options,
                Err(_) => return unavailable(),
            };
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            let mut data = rendering::appearance_context();
            data["title"] = json!("Medication created");
            data["slug"] = json!(slug);
            data["id"] = json!(id);
            data["name"] = json!(name);
            data["dose_summaries"] = json!(
                options
                    .into_iter()
                    .map(|option| format!(
                        "{} {} · {}",
                        option.amount.normalize(),
                        option.unit,
                        option.frequency
                    ))
                    .collect::<Vec<_>>()
            );
            match format::render().view(&view, "medications/created.html", data) {
                Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
                Err(_) => unavailable(),
            }
        }
        Err(error) => {
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            open(
                &ctx,
                &session,
                &token,
                &view,
                WizardRequest {
                    slug: &slug,
                    request_id,
                    draft,
                    error: Some(&error),
                },
            )
            .await
        }
    }
}

async fn save(
    tenant: &TenantTransaction,
    principal: &BrowserPrincipal,
    draft: &HashMap<String, String>,
) -> std::result::Result<(i64, String), OperationError> {
    let value = |key| forms::field(draft, key);
    let candidate =
        json!({"barcode":value("barcode"),"code":value("dmd_code"),"name":value("name")});
    medications::lock_row(
        tenant.transaction(),
        "households",
        tenant.scope().household_id,
    )
    .await?;
    if !medication_lookup::stock_matches(tenant, &candidate)
        .await?
        .is_empty()
    {
        return Err(OperationError::Validation {
            details: json!({"error":"A matching medication is already in this household. Return to Medication Finder and choose the medicine and location to refill."}),
        });
    }
    let mut attrs = serde_json::Map::new();
    for key in [
        "name",
        "friendly_name",
        "description",
        "warnings",
        "dose_unit",
        "reorder_threshold",
        "barcode",
        "dmd_code",
        "dmd_system",
        "dmd_concept_class",
        "category",
        "default_schedule_type",
    ] {
        if let Some(value) = draft.get(key) {
            if key == "dmd_system"
                && value == "https://dmd.nhs.uk"
                && draft
                    .get("dmd_code")
                    .is_none_or(|code| code.trim().is_empty())
            {
                continue;
            }
            attrs.insert(key.into(), json!(value));
        }
    }
    attrs.insert(
        "location_id".into(),
        json!(value("location_id").parse::<i64>().unwrap_or_default()),
    );
    for key in ["dose_amount", "current_supply"] {
        attrs.insert(
            key.into(),
            if value(key).is_empty() {
                Value::Null
            } else {
                json!(value(key))
            },
        );
    }
    attrs.insert(
        "default_schedule_type".into(),
        json!(value("schedule_type")),
    );
    let config = schedule_defaults(draft)?;
    let record = medications::crud::create_with_schedule_config(
        tenant,
        Value::Object(attrs),
        Some(config),
        Some(principal.provenance()),
    )
    .await?;
    let mut plan = draft.clone();
    plan.insert("medication_id".into(), record.id.to_string());
    if !value("dose_amount").is_empty() {
        let body = json!({"dosage_option":{"medication_id":record.id.to_string(),"amount":value("dose_amount"),"unit":value("dose_unit"),"frequency":value("frequency"),"default_max_daily_doses":value("max_daily_doses").parse::<i32>().unwrap_or(2),"default_min_hours_between_doses":value("min_hours_between_doses"),"default_dose_cycle":value("dose_cycle"),"current_supply":if value("current_supply").is_empty(){Value::Null}else{json!(value("current_supply"))},"reorder_threshold":value("reorder_threshold")}});
        let (created, _) =
            dosages::create_with_parent(tenant, body, record.clone(), Some(principal.provenance()))
                .await?;
        if let Some(id) = created["id"].as_i64() {
            plan.insert("source_dosage_option_id".into(), id.to_string());
        }
    }
    if !value("extra_options").is_empty() {
        let options: Vec<Value> = serde_json::from_str(value("extra_options")).map_err(|_| {
            OperationError::Validation {
                details: json!({"error":"Dose options are invalid."}),
            }
        })?;
        if options.len() > 10 || options.iter().any(|option| !option.is_object()) {
            return Err(OperationError::Validation {
                details: json!({"error":"Dose options are invalid."}),
            });
        }
        for option in options {
            let mut attrs = option.as_object().cloned().unwrap_or_default();
            attrs.insert("medication_id".into(), json!(record.id.to_string()));
            for key in [
                "amount",
                "default_min_hours_between_doses",
                "current_supply",
                "reorder_threshold",
            ] {
                if let Some(Value::Number(number)) = attrs.get(key) {
                    attrs.insert(key.into(), json!(number.to_string()));
                }
            }
            for key in ["current_supply", "reorder_threshold"] {
                if attrs.get(key).and_then(Value::as_str) == Some("") {
                    attrs.insert(key.into(), Value::Null);
                }
            }
            let body = json!({"dosage_option": attrs});
            dosages::create_with_parent(tenant, body, record.clone(), Some(principal.provenance()))
                .await?;
        }
    }
    if !value("person_id").is_empty() {
        let supplement = matches!(
            value("category").to_ascii_lowercase().as_str(),
            "vitamin" | "mineral" | "supplement"
        );
        let direct = supplement || value("schedule_type") == "prn";
        if direct {
            plan.insert(
                "administration_kind".into(),
                if supplement { "routine" } else { "as_needed" }.into(),
            );
        }
        let body = crate::controllers::treatments::forms::body(
            if direct {
                crate::controllers::treatments::Kind::Assignment
            } else {
                crate::controllers::treatments::Kind::Schedule
            },
            value("person_id"),
            &plan,
            None,
        )?;
        if direct {
            assignments::create(tenant, &body, Some(principal.provenance())).await?;
        } else {
            treatments::create(tenant, &body, Some(principal.provenance())).await?;
        }
    }
    Ok((record.id, record.name.unwrap_or_default()))
}

fn schedule_defaults(draft: &HashMap<String, String>) -> Result<Value, OperationError> {
    let value = |key| forms::field(draft, key);
    let schedule_type = value("schedule_type");
    if ![
        "daily",
        "multiple_daily",
        "weekly",
        "specific_dates",
        "prn",
        "tapering",
    ]
    .contains(&schedule_type)
    {
        return Err(OperationError::Validation {
            details: json!({"errors":{"schedule_type":["is invalid"]}}),
        });
    }
    let frequency = value("frequency");
    if frequency.len() > 255 {
        return Err(OperationError::Validation {
            details: json!({"errors":{"frequency":["is too long"]}}),
        });
    }
    let mut config = crate::controllers::treatments::forms::configuration(draft, None)?;
    for (field, format) in [("times", "%H:%M"), ("dates", "%Y-%m-%d")] {
        if config[field].as_array().is_some_and(|values| {
            values.iter().any(|value| {
                value.as_str().is_none_or(|value| {
                    if field == "times" {
                        chrono::NaiveTime::parse_from_str(value, format).is_err()
                    } else {
                        chrono::NaiveDate::parse_from_str(value, format).is_err()
                    }
                })
            })
        }) {
            return Err(OperationError::Validation {
                details: json!({"errors":{field:["is invalid"]}}),
            });
        }
    }
    config["schedule_type"] = json!(schedule_type);
    config["frequency"] = json!(frequency);
    Ok(config)
}
