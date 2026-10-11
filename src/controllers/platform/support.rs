use axum::extract::Path;

use super::*;

fn sessions_json(sessions: &[platform::support::SupportEntry]) -> Vec<Value> {
    sessions
        .iter()
        .map(|session| {
            json!({
                "id": session.id,
                "household_id": session.household_id,
                "household_name": session.household_name,
                "reason": session.reason,
                "requester": session.requester,
                "state": session.state,
                "expires_at": session.expires_at,
            })
        })
        .collect()
}

fn render_index(
    view: &TeraView,
    token: &CsrfToken,
    page: &platform::support::SupportPage,
) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Support access");
    data["sessions"] = json!(sessions_json(&page.sessions));
    data["page"] = json!(page.page);
    data["pages"] = json!(page.pages);
    data["page_links"] = json!(page.page_links);
    data["households"] = json!(
        page.households
            .iter()
            .map(|household| json!({"id": household.id, "name": household.name}))
            .collect::<Vec<Value>>()
    );
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(view, "platform/support.html", data) {
        Ok(response) => (
            StatusCode::OK,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

fn render_read(
    view: &TeraView,
    token: &CsrfToken,
    read: &platform::support::SupportRead,
) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Support access");
    data["household_name"] = json!(read.household_name);
    data["slug"] = json!(read.slug);
    data["expires_at"] = json!(read.expires_at);
    data["members"] = json!(read.members);
    data["people"] = json!(read.people);
    data["medications"] = json!(read.medications);
    data["allocations"] = json!(
        read.allocations
            .iter()
            .map(|allocation| json!({
                "person": allocation.person,
                "medication": allocation.medication,
                "dose": allocation.dose,
                "max_daily": allocation.max_daily,
            }))
            .collect::<Vec<Value>>()
    );
    data["schedules"] = json!(
        read.schedules
            .iter()
            .map(|schedule| json!({
                "person": schedule.person,
                "medication": schedule.medication,
                "frequency": schedule.frequency,
                "dose": schedule.dose,
            }))
            .collect::<Vec<Value>>()
    );
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(view, "platform/support_read.html", data) {
        Ok(response) => (
            StatusCode::OK,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

async fn platform_account(
    service: &IdentityService,
    headers: &HeaderMap,
) -> Result<(i64, better_auth_core::wire::SessionView), Box<Response>> {
    let identity = browser_identity(service, &session_request(headers))
        .await
        .map_err(|_| Box::new(unavailable()))?;
    let BrowserIdentity::Authenticated { user, session } = identity else {
        return Err(Box::new(Redirect::to("/login").into_response()));
    };
    let account_id = clinical_id(&user.id).map_err(|_| Box::new(unavailable()))?;
    Ok((account_id, session))
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Query(params): Query<UsersQuery>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let (account_id, session) = match platform_account(&service, &headers).await {
        Ok(account) => account,
        Err(response) => return *response,
    };
    let Ok(transaction) = platform::begin(&ctx.db).await else {
        return unavailable();
    };
    let meta = browser_meta(&session, request_id, &headers);
    let page =
        match platform::support::page(&transaction, account_id, &meta, params.page.unwrap_or(1))
            .await
        {
            Ok(page) => page,
            Err(error) => {
                let unauthenticated = matches!(error, OperationError::Unauthenticated);
                if transaction.rollback().await.is_err() {
                    return unavailable();
                }
                return if unauthenticated {
                    Redirect::to("/login").into_response()
                } else {
                    operation_error(error)
                };
            }
        };
    let response = render_index(&view, &token, &page);
    if transaction.commit().await.is_err() {
        return unavailable();
    }
    response
}

#[derive(Deserialize)]
pub(super) struct RequestForm {
    household_id: i64,
    reason: String,
    authenticity_token: String,
}

pub(super) async fn request(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<RequestForm>,
) -> Response {
    proof(
        &service,
        &headers,
        request_id,
        token,
        &view,
        &form.authenticity_token,
        (
            json!({
                "action": "support_request",
                "household_id": form.household_id,
                "reason": form.reason,
            }),
            "support access request",
        ),
    )
    .await
}

#[derive(Deserialize)]
pub(super) struct SupportForm {
    support_id: i64,
    authenticity_token: String,
}

pub(super) async fn activate(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<SupportForm>,
) -> Response {
    proof(
        &service,
        &headers,
        request_id,
        token,
        &view,
        &form.authenticity_token,
        (
            json!({"action": "support_activate", "support_id": form.support_id}),
            "support access activation",
        ),
    )
    .await
}

pub(super) async fn end(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    Form(form): Form<SupportEndForm>,
) -> Response {
    end_support(
        &ctx,
        &service,
        &headers,
        request_id,
        token,
        form.support_id,
        &form.authenticity_token,
        "/platform/support",
    )
    .await
}

pub(super) async fn read(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Path(support_id): Path<i64>,
) -> Response {
    let (account_id, session) = match platform_account(&service, &headers).await {
        Ok(account) => account,
        Err(response) => return *response,
    };
    let Ok(transaction) = platform::begin(&ctx.db).await else {
        return unavailable();
    };
    let meta = browser_meta(&session, request_id, &headers);
    match platform::support::read(&transaction, account_id, support_id, &meta).await {
        Ok(read) => {
            let response = render_read(&view, &token, &read);
            if transaction.commit().await.is_err() {
                return unavailable();
            }
            response
        }
        Err(error) => {
            let unauthenticated = matches!(error, OperationError::Unauthenticated);
            let persisted = matches!(error, OperationError::Forbidden);
            let ended = if persisted {
                transaction.commit().await
            } else {
                transaction.rollback().await
            };
            if ended.is_err() {
                return unavailable();
            }
            if unauthenticated {
                Redirect::to("/login").into_response()
            } else {
                operation_error(error)
            }
        }
    }
}
