mod form;

use super::medications::{
    authentication_error, begin, forms, operation_error, rendering, request_id, unavailable,
};
use crate::models::{
    access::TenantTransaction,
    care::{dosages, medications},
    errors::OperationError,
};
use axum::{
    Extension,
    extract::Form as AxumForm,
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households/{slug}/medications/{medication}/dosage_options")
        .add("/", get(index).post(create))
        .add("/new", get(new))
        .add("/{id}/edit", get(edit))
        .add("/{id}", post(update))
        .add("/{id}/destroy", post(destroy))
}

struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    medication: &'a str,
    request_id: String,
}

impl Page<'_> {
    async fn open(&self, id: Option<&str>, listing: bool) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let response = match self.data(&tenant, id, listing).await {
            Ok(data) => self.render(data, listing, StatusCode::OK),
            Err(error) => return operation_error(error),
        };
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        response
    }

    async fn data(
        &self,
        tenant: &TenantTransaction,
        id: Option<&str>,
        listing: bool,
    ) -> std::result::Result<Value, OperationError> {
        if !dosages::can_manage(tenant).await? {
            return Err(OperationError::Forbidden);
        }
        let parent = medications::read_stock_snapshot(tenant, self.medication).await?;
        let base = format!(
            "/households/{}/medications/{}/dosage_options",
            self.slug, parent.medication.id
        );
        let mut data = rendering::appearance_context();
        data["title"] = json!(if listing {
            "Dose Options"
        } else if id.is_some() {
            "Edit Dose Option"
        } else {
            "Add Dose Option"
        });
        data["base"] = json!(base);
        data["medication_url"] = json!(format!(
            "/households/{}/medications/{}",
            self.slug, parent.medication.id
        ));
        data["medication_name"] = json!(parent.representation["data"]["display_name"]);
        data["error"] = Value::Null;
        data["errors"] = json!({});
        data["changed"] = json!(false);
        if listing {
            data["options"] =
                json!(dosages::for_medication(tenant, &parent.medication.id.to_string()).await?);
        } else {
            let (record, etag) = match id {
                Some(id) => {
                    let (body, etag) = dosages::read(tenant, id).await?;
                    if body["data"]["medication_id"].as_i64() != Some(parent.medication.id) {
                        return Err(OperationError::NotFound);
                    }
                    (body["data"].clone(), etag)
                }
                None => (form::defaults(), String::new()),
            };
            data["action"] = json!(id.map_or_else(|| base.clone(), |id| format!("{base}/{id}")));
            data["latest_url"] = json!(id.map(|id| format!("{base}/{id}/edit")));
            data["draft"] = json!(form::draft(&record, &etag));
        }
        Ok(data)
    }

    fn render(&self, mut data: Value, listing: bool, status: StatusCode) -> Response {
        let Ok(token) = self.token.authenticity_token() else {
            return unavailable();
        };
        data["authenticity_token"] = json!(token);
        data["units"] = json!([
            "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet",
            "pad"
        ]);
        data["cycles"] = json!(["daily", "weekly", "monthly"]);
        if !listing {
            data["dose_fields"] =
                form::fields(&data["draft"], &data["errors"], &["amount", "description"]);
            data["timing_fields"] = form::fields(
                &data["draft"],
                &data["errors"],
                &[
                    "frequency",
                    "default_max_daily_doses",
                    "default_min_hours_between_doses",
                ],
            );
            data["stock_fields"] = form::fields(
                &data["draft"],
                &data["errors"],
                &["current_supply", "reorder_threshold"],
            );
        }
        let template = if listing {
            "dosage_options/index.html"
        } else {
            "dosage_options/form.html"
        };
        match format::render().view(self.view, template, data) {
            Ok(mut response) => {
                *response.status_mut() = status;
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-store"),
                );
                response
            }
            Err(_) => unavailable(),
        }
    }

    async fn save(
        &self,
        id: Option<&str>,
        draft: HashMap<String, String>,
        deleting: bool,
    ) -> Response {
        if self
            .token
            .verify(forms::field(&draft, "authenticity_token"))
            .is_err()
        {
            return operation_error(OperationError::Forbidden);
        }
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let result = async {
            let parent = medications::read_stock_snapshot(&tenant, self.medication).await?;
            if let Some(id) = id {
                let (body, current_etag) = dosages::read(&tenant, id).await?;
                if body["data"]["medication_id"].as_i64() != Some(parent.medication.id) {
                    return Err(OperationError::NotFound);
                }
                if deleting {
                    return dosages::destroy(&tenant, id, Some(principal.provenance())).await;
                }
                if forms::field(&draft, "etag") != current_etag {
                    return Err(OperationError::Conflict {
                        code: "conflict".into(),
                        details: json!({}),
                    });
                }
                if !form::browser_hours_valid(&draft, body["data"].get("default_min_hours_between_doses")) {
                    return Err(OperationError::Validation {
                        details: json!({"error":"Minimum hours between doses must be a whole number.","errors":{"default_min_hours_between_doses":["must be a whole number"]}}),
                    });
                }
                dosages::update(
                    &tenant,
                    id,
                    form::attributes(&draft, None),
                    Some(forms::field(&draft, "etag")),
                    Some(principal.provenance()),
                )
                .await?;
            } else {
                if !form::browser_hours_valid(&draft, None) {
                    return Err(OperationError::Validation {
                        details: json!({"error":"Minimum hours between doses must be a whole number.","errors":{"default_min_hours_between_doses":["must be a whole number"]}}),
                    });
                }
                dosages::create(
                    &tenant,
                    form::attributes(&draft, Some(parent.medication.id)),
                    Some(principal.provenance()),
                )
                .await?;
            }
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {
                if tenant.commit().await.is_err() {
                    return unavailable();
                }
                (
                    StatusCode::SEE_OTHER,
                    [
                        (
                            header::LOCATION,
                            format!(
                                "/households/{}/medications/{}/dosage_options",
                                self.slug, self.medication
                            ),
                        ),
                        (header::CACHE_CONTROL, "no-store".into()),
                    ],
                )
                    .into_response()
            }
            Err(error) => {
                if tenant.rollback().await.is_err() {
                    return unavailable();
                }
                let tenant = match principal
                    .begin_household_slug(&self.ctx.db, self.slug, self.request_id.clone())
                    .await
                {
                    Ok(value) => value,
                    Err(error) => return authentication_error(error),
                };
                let mut data = match self.data(&tenant, id, deleting).await {
                    Ok(value) => value,
                    Err(error) => return operation_error(error),
                };
                if !deleting {
                    data["draft"] = json!(draft);
                }
                data["error"] = json!(forms::message(&error));
                if let OperationError::Validation { details } = &error {
                    data["errors"] = details["errors"].clone();
                }
                data["changed"] = json!(matches!(error, OperationError::Conflict { .. }));
                let response = self.render(data, deleting, forms::status(&error));
                if tenant.commit().await.is_err() {
                    return unavailable();
                }
                response
            }
        }
    }
}

async fn index(
    State(ctx): State<AppContext>,
    Path((slug, medication)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        medication: &medication,
        request_id: request_id(request),
    }
    .open(None, true)
    .await
}
async fn new(
    State(ctx): State<AppContext>,
    Path((slug, medication)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        medication: &medication,
        request_id: request_id(request),
    }
    .open(None, false)
    .await
}
async fn edit(
    State(ctx): State<AppContext>,
    Path((slug, medication, id)): Path<(String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        medication: &medication,
        request_id: request_id(request),
    }
    .open(Some(&id), false)
    .await
}
async fn create(
    State(ctx): State<AppContext>,
    Path((slug, medication)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        medication: &medication,
        request_id: request_id(request),
    }
    .save(None, draft, false)
    .await
}
async fn update(
    State(ctx): State<AppContext>,
    Path((slug, medication, id)): Path<(String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        medication: &medication,
        request_id: request_id(request),
    }
    .save(Some(&id), draft, false)
    .await
}
async fn destroy(
    State(ctx): State<AppContext>,
    Path((slug, medication, id)): Path<(String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        medication: &medication,
        request_id: request_id(request),
    }
    .save(Some(&id), draft, true)
    .await
}
