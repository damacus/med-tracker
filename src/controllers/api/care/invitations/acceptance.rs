use super::*;
use crate::models::identity::api_session;

pub(in crate::controllers::api::care) async fn accept(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let principal = match api_session::authenticate(&ctx.db, &headers).await {
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
    let response=match invitations::accept(&ctx.db,&principal,token,&request_id).await {
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
