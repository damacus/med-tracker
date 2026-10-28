use better_auth_core::{AuthRecordFields, AuthRequest, HttpMethod, session::NativeSessionData};
use loco_rs::app::AppContext;
use med_tracker::models::identity::better_auth::IdentityService;

pub async fn browser_cookie(context: &AppContext, account_id: i64) -> String {
    let service = context.shared_store.get::<IdentityService>().unwrap();
    let auth = service.context();
    let account_id = account_id.to_string();
    let user = auth
        .database
        .get_user_by_id(&account_id)
        .await
        .unwrap()
        .expect("browser fixture account must exist");
    let manager = auth.session_manager();
    let session = manager
        .create_session_for_id(account_id.into(), None, None)
        .await
        .unwrap();
    let request = AuthRequest::new(HttpMethod::Get, "/get-session");
    manager
        .set_native_session_cookie(
            &request,
            NativeSessionData::new(session, user.field_values().unwrap().into()),
            Some(false),
        )
        .await
        .unwrap();
    let headers = request.take_response_headers().unwrap();
    let cookie = headers
        .get_all("set-cookie")
        .map(|value| value.split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ");
    assert!(!cookie.is_empty(), "Better Auth must issue browser cookies");
    cookie
}
