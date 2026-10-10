use axum::{extract::Request, middleware::Next, response::Response};
use loco_rs::prelude::*;
use serde_json::{Value, json};

tokio::task_local! {
    static LANGUAGE: &'static str;
}

pub(super) async fn scope(request: Request, next: Next) -> Response {
    let language = crate::models::care::report_pdf::locale(
        request
            .headers()
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    LANGUAGE.scope(language, next.run(request)).await
}

pub(super) fn view(
    view: &TeraView,
    path: &str,
    mut data: Value,
) -> std::result::Result<Response, Box<Error>> {
    let language = LANGUAGE.try_with(|language| *language).unwrap_or("en");
    let labels = crate::models::care::report_pdf::translations(language)
        .map_err(|_| Error::string("Security translations unavailable"))?;
    let english = crate::models::care::report_pdf::translations("en")
        .map_err(|_| Error::string("Security translations unavailable"))?;
    let flow = &labels["profiles"]["security_flow"];
    let source = english["profiles"]["security_flow"]
        .as_object()
        .ok_or_else(|| Error::string("Security translations unavailable"))?;
    for field in ["title", "error", "operation_label"] {
        if let Some(original) = data[field].as_str()
            && let Some((key, _)) = source
                .iter()
                .find(|(_, value)| value.as_str() == Some(original))
        {
            let target = if field == "operation_label" {
                "operation_text"
            } else {
                field
            };
            data[target] = flow[key].clone();
        }
    }
    data["flow"] = flow.clone();
    data["flow_json"] = json!(flow.to_string());
    data["lang"] = json!(language);
    format::render().view(view, path, data).map_err(Box::new)
}
