use super::*;

fn household_json(household: &platform::recovery::RecoveryHousehold) -> Value {
    json!({
        "id": household.id,
        "name": household.name,
        "slug": household.slug,
        "has_owner": household.has_owner,
        "members": household.members.iter().map(|member| json!({
            "id": member.id,
            "name": member.name,
            "email": member.email,
        })).collect::<Vec<Value>>(),
    })
}

fn render(
    view: &TeraView,
    token: &CsrfToken,
    households: &[platform::recovery::RecoveryHousehold],
    selected: Option<&platform::recovery::RecoveryHousehold>,
    search: &str,
) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Owner recovery");
    data["search"] = json!(search);
    data["households"] = json!(
        households
            .iter()
            .map(household_json)
            .collect::<Vec<Value>>()
    );
    data["selected"] = json!(selected.map(household_json));
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(view, "platform/owner_recovery.html", data) {
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

#[derive(Deserialize)]
pub(super) struct RecoveryQuery {
    household: Option<i64>,
    search: Option<String>,
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<RecoveryQuery>,
) -> Response {
    let identity = match browser_identity(&service, &session_request(&headers)).await {
        Ok(identity) => identity,
        Err(_) => return unavailable(),
    };
    let BrowserIdentity::Authenticated { user, .. } = identity else {
        return Redirect::to("/login").into_response();
    };
    let Ok(account_id) = clinical_id(&user.id) else {
        return unavailable();
    };
    let Ok(transaction) = platform::begin(&ctx.db).await else {
        return unavailable();
    };
    let page = async || -> Result<(Vec<platform::recovery::RecoveryHousehold>, Option<platform::recovery::RecoveryHousehold>), OperationError> {
        let households =
            platform::recovery::page(&transaction, account_id, query.search.as_deref()).await?;
        let selected = match query.household {
            Some(household_id) => {
                platform::recovery::detail(&transaction, account_id, household_id).await?
            }
            None => None,
        };
        Ok((households, selected))
    };
    let (households, selected) = match page().await {
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
    let response = render(
        &view,
        &token,
        &households,
        selected.as_ref(),
        query.search.as_deref().unwrap_or_default(),
    );
    if transaction.commit().await.is_err() {
        return unavailable();
    }
    response
}

#[derive(Deserialize)]
pub(super) struct RecoveryForm {
    household_id: i64,
    membership_id: i64,
    reason: String,
    authenticity_token: String,
}

pub(super) async fn submit(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<RecoveryForm>,
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
                "action": "platform_owner_recovery",
                "household_id": form.household_id,
                "membership_id": form.membership_id,
                "reason": form.reason,
            }),
            "household owner recovery",
        ),
    )
    .await
}
