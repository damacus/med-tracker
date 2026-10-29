use super::administration::request_id;
use super::*;
use crate::models::{
    care::person_schedules::{self, projection},
    ids::MedicationId,
};

pub(super) async fn index(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = projection::list(&tenant)
        .await
        .and_then(|plans| {
            plans
                .iter()
                .map(person_schedules::SharedPlan::try_from)
                .collect::<std::result::Result<Vec<_>, _>>()
        })
        .map(|plans| (StatusCode::OK, json!({"data":plans}), None))
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::treatment("GET", "shared_index"),
        result,
        &request_id,
    )
    .await
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StockBindingInput {
    medication_id: i64,
    clinical_revision: i64,
}

pub(super) async fn bind(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<StockBindingInput>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = async {
        let AxumJson(input) =
            body.map_err(|_| response::Failure::bad_request("Invalid stock binding"))?;
        let command = projection::BindStock {
            plan: person_schedules::PersonScheduleId::parse(&id).map_err(response::operation)?,
            revision: person_schedules::ClinicalRevision::parse(input.clinical_revision)
                .map_err(response::operation)?,
            medication: MedicationId::from_database(input.medication_id),
        };
        let row = projection::bind(&tenant, command, principal.provenance())
            .await
            .map_err(response::operation)?;
        let (body, etag) = crate::models::care::treatments::representation(&row);
        Ok((StatusCode::OK, body, Some(etag)))
    }
    .await;
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::treatment("POST", "bind_stock"),
        result,
        &request_id,
    )
    .await
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MedicineInput {
    medication_id: i64,
    medication_etag: String,
    dose_amount: String,
    dose_unit: String,
}

pub(super) async fn medicine(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<MedicineInput>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = async {
        let AxumJson(input) = body.map_err(|_| {
            response::operation(crate::models::profile::validation(
                "medication",
                "Choose a medicine and confirm its prescribed dose and unit",
            ))
        })?;
        let id = person_schedules::PersonScheduleId::parse(&id).map_err(response::operation)?;
        let etag = headers
            .get(axum::http::header::IF_MATCH)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                response::operation(crate::models::profile::validation(
                    "if_match",
                    "is required",
                ))
            })?;
        let expected = person_schedules::ClinicalRevision::from_etag(&id, etag)
            .map_err(response::operation)?;
        let choice = person_schedules::shared_commands::MedicineChange::parse(
            input.medication_id,
            input.medication_etag,
            &input.dose_amount,
            input.dose_unit,
        )
        .map_err(response::operation)?;
        let plan =
            person_schedules::shared_commands::medicine(&tenant, &principal, id, expected, choice)
                .await
                .map_err(|error| match error {
                    person_schedules::shared_commands::Failure::Operation(error) => {
                        response::operation(error)
                    }
                    person_schedules::shared_commands::Failure::Authentication(error) => {
                        response::authentication(error)
                    }
                })?;
        let etag = plan.etag.clone();
        Ok((StatusCode::OK, json!({"data":plan}), Some(etag)))
    }
    .await;
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::treatment("PATCH", "change_medicine"),
        result,
        &request_id,
    )
    .await
}
