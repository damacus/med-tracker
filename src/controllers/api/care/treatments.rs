use super::administration::{complete, key, request_id, store};
use super::*;
use crate::models::care::{locations::SavedResponse, treatments};

pub(super) async fn create(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    #[cfg(debug_assertions)]
    let diagnostic_start = std::time::Instant::now();
    #[cfg(debug_assertions)]
    eprintln!("schedule diagnostic tenant begin start");
    let request_id = request_id(request);
    let tenant_result = begin(&ctx, &headers, household_id, &request_id).await;
    #[cfg(debug_assertions)]
    eprintln!(
        "schedule diagnostic tenant begin end {:?}",
        diagnostic_start.elapsed()
    );
    let (principal, tenant) = match tenant_result {
        Ok(value) => value,
        Err(error) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "schedule diagnostic handler end {:?}",
                diagnostic_start.elapsed()
            );
            return response::error(error, &request_id);
        }
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "schedule diagnostic handler end {:?}",
                diagnostic_start.elapsed()
            );
            return response::error(response::unavailable(), &request_id);
        }
    };
    let result = async {
        let AxumJson(body) =
            body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
        #[cfg(debug_assertions)]
        eprintln!(
            "schedule diagnostic first authorization start {:?}",
            diagnostic_start.elapsed()
        );
        let authorization = treatments::authorize_person(&tenant, &body).await;
        #[cfg(debug_assertions)]
        eprintln!(
            "schedule diagnostic first authorization end {:?}",
            diagnostic_start.elapsed()
        );
        authorization.map_err(response::operation)?;
        let path = format!("/api/v1/households/{household_id}/schedules");
        let key = key(&headers);
        if let Some(key) = key {
            #[cfg(debug_assertions)]
            eprintln!(
                "schedule diagnostic idempotency start {:?}",
                diagnostic_start.elapsed()
            );
            let replay = locations::keyed_replay(&tenant, key, "POST", &path, &body).await;
            #[cfg(debug_assertions)]
            eprintln!(
                "schedule diagnostic idempotency end {:?}",
                diagnostic_start.elapsed()
            );
            if let Some(reply) = replay? {
                return Ok(reply);
            }
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "schedule diagnostic create start {:?}",
            diagnostic_start.elapsed()
        );
        let created = treatments::create(&tenant, &body, Some(principal.provenance())).await;
        #[cfg(debug_assertions)]
        eprintln!(
            "schedule diagnostic create end {:?}",
            diagnostic_start.elapsed()
        );
        let (reply, etag) = created.map_err(response::operation)?;
        if let Some(key) = key {
            #[cfg(debug_assertions)]
            eprintln!(
                "schedule diagnostic idempotency store start {:?}",
                diagnostic_start.elapsed()
            );
            let stored = store(
                &tenant,
                &principal,
                SavedResponse {
                    key,
                    method: "POST",
                    path: &path,
                    request: &body,
                    status: 201,
                    body: reply,
                    etag: Some(&etag),
                },
            )
            .await;
            #[cfg(debug_assertions)]
            eprintln!(
                "schedule diagnostic idempotency store end {:?}",
                diagnostic_start.elapsed()
            );
            return stored;
        }
        Ok((StatusCode::CREATED, reply, Some(etag), false))
    }
    .await;
    let restored = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if restored.is_err() {
        #[cfg(debug_assertions)]
        eprintln!(
            "schedule diagnostic handler end {:?}",
            diagnostic_start.elapsed()
        );
        return response::error(response::unavailable(), &request_id);
    }
    #[cfg(debug_assertions)]
    eprintln!(
        "schedule diagnostic completion start {:?}",
        diagnostic_start.elapsed()
    );
    let response = complete(
        tenant,
        &principal,
        audit::RequestAudit::treatment("POST", "create"),
        result,
        &request_id,
    )
    .await;
    #[cfg(debug_assertions)]
    eprintln!(
        "schedule diagnostic handler end {:?}",
        diagnostic_start.elapsed()
    );
    response
}
