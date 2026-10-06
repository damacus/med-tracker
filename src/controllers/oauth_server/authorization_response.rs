use crate::models::identity::authorization::{
    AuthorizationFailure, AuthorizationInput, AuthorizationResponse,
};
use axum::{
    http::{StatusCode, header},
    response::IntoResponse,
};
use loco_rs::prelude::*;
use oxide_auth::code_grant::authorization::Error;

pub(super) fn render(
    view: &TeraView,
    input: &AuthorizationInput,
    output: AuthorizationResponse,
) -> Response {
    if input.value("response_mode") == Some("query") {
        return (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, output.response.as_str()),
                (header::CACHE_CONTROL, "no-store"),
                (header::PRAGMA, "no-cache"),
            ],
        )
            .into_response();
    }
    let mut base = output.response.clone();
    base.set_query(output.callback.query());
    if base != output.callback {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let original = output.callback.query_pairs().collect::<Vec<_>>();
    let generated = output.response.query_pairs().collect::<Vec<_>>();
    if !generated.starts_with(&original) {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let fields = generated
        .into_iter()
        .skip(original.len())
        .map(|(name, value)| serde_json::json!({"name":name,"value":value}))
        .collect::<Vec<_>>();
    match format::render().view(
        view,
        "oauth/form_post.html",
        serde_json::json!({"callback":output.callback.as_str(),"fields":fields}),
    ) {
        Ok(response) => (
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::PRAGMA, "no-cache"),
                (header::REFERRER_POLICY, "no-referrer"),
            ],
            response,
        )
            .into_response(),
        Err(error) => error.into_response(),
    }
}

pub(super) fn failure(
    view: &TeraView,
    input: &AuthorizationInput,
    failure: AuthorizationFailure,
) -> Response {
    match *failure.error {
        Error::Redirect(error) => match failure.callback {
            Some(callback) => render(
                view,
                input,
                AuthorizationResponse {
                    callback,
                    response: error.into(),
                },
            ),
            None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        },
        Error::Ignore => StatusCode::BAD_REQUEST.into_response(),
        Error::PrimitiveError => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
