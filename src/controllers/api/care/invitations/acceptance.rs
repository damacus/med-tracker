use super::*;
use crate::models::identity::api_session;

pub(in crate::controllers::api::care) async fn accept(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    dependent: Option<Extension<super::DependentSharing>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match api_session::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => {
                return no_store(response::error(
                    response::authentication(error),
                    &request_id,
                ));
            }
        };
    let AxumJson(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return no_store(response::error(
                response::Failure::bad_request("Invalid request body"),
                &request_id,
            ));
        }
    };
    let token = match body
        .get("token")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
    {
        Some(token) => token,
        None => {
            return no_store(response::error(
                response::Failure::bad_request("Invalid request body"),
                &request_id,
            ));
        }
    };
    if body.get("person_id").is_some()
        || (dependent.is_none() && body.get("subject_person_id").is_some())
        || body.get("dependent_ids").is_some()
    {
        return no_store(response::error(
            response::Failure::bad_request("Self invitations cannot select another person"),
            &request_id,
        ));
    }
    let result = if dependent.is_some() {
        let consent = match invitations::SharingConsent::parse(
            body.get("sharing_consent").and_then(Value::as_bool),
        ) {
            Ok(consent) => consent,
            Err(error) => {
                return no_store(response::error(response::operation(error), &request_id));
            }
        };
        match body
            .get("subject_person_id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
        {
            Some(id) => {
                invitations::acceptance::accept_dependent(
                    &ctx.db,
                    &principal,
                    token,
                    consent,
                    crate::models::ids::PersonId::from_database(id),
                    &request_id,
                    ctx.config.settings.as_ref(),
                )
                .await
            }
            None => Err(OperationError::NotFound),
        }
    } else {
        let consent = invitations::SharingConsent::parse(
            body.get("sharing_consent").and_then(Value::as_bool),
        )
        .ok();
        invitations::accept(
            &ctx.db,
            &principal,
            token,
            consent,
            &request_id,
            ctx.config.settings.as_ref(),
        )
        .await
    };
    let response=match result {
        Ok(body)=>(StatusCode::OK,AxumJson(body)).into_response(),
        Err(OperationError::Validation{..})=>(StatusCode::UNPROCESSABLE_ENTITY,AxumJson(json!({"error":{"code":"invitation_unavailable","message":"Invitation is unavailable","request_id":request_id}}))).into_response(),
        Err(error)=>response::error(response::operation(error),&request_id),
    };
    let mut response = no_store(response);
    if let Ok(value) = axum::http::HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}
