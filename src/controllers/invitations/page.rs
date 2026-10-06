use super::*;

#[derive(Clone, Copy)]
pub(super) enum Change {
    Create,
    Resend(i64),
    Cancel(i64),
}

pub(super) struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    request_id: String,
}

impl<'a> Page<'a> {
    pub(super) fn new(
        ctx: &'a AppContext,
        session: &'a Session<SessionPgPool>,
        token: &'a CsrfToken,
        view: &'a TeraView,
        slug: &'a str,
        request: Option<Extension<LocoRequestId>>,
    ) -> Self {
        Self {
            ctx,
            session,
            token,
            view,
            slug,
            request_id: request_id(request),
        }
    }

    pub(super) async fn index(
        &self,
        submitted: Option<&[(String, String)]>,
        error: Option<&OperationError>,
    ) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let result = async {
            let records = invitations::list(&tenant).await?;
            let options = invitations::options(&tenant).await?;
            Ok::<_, OperationError>(rendering::data(
                self.slug, &records, &options, submitted, error,
            ))
        }
        .await;
        let data = match result {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
        let response = rendering::render(self.view, self.token, data, error);
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        response
    }

    pub(super) async fn save(&self, change: Change, draft: Vec<(String, String)>) -> Response {
        if self
            .token
            .verify(field(&draft, "authenticity_token"))
            .is_err()
        {
            return operation_error(OperationError::Forbidden);
        }
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let target = match url::Url::parse(&self.ctx.config.server.full_url())
            .and_then(|url| url.join("/invitations/accept"))
        {
            Ok(url) => url,
            Err(_) => return unavailable(),
        };
        let result = match change {
            Change::Create => invitations::create(
                &tenant,
                attributes(&draft),
                Some(&target),
                Some(principal.provenance()),
            )
            .await
            .map(|_| ()),
            Change::Cancel(id) => {
                invitations::cancel(&tenant, id, Some(principal.provenance())).await
            }
            Change::Resend(id) => {
                invitations::resend(&tenant, id, &target, Some(principal.provenance()))
                    .await
                    .map(|_| ())
                    .map_err(|error| match error {
                        invitations::ResendError::Operation(error) => error,
                        invitations::ResendError::DeliveryUnavailable => {
                            OperationError::Unavailable
                        }
                    })
            }
        };
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
                            format!("/households/{}/admin/invitations", self.slug),
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
                if matches!(
                    error,
                    OperationError::Forbidden
                        | OperationError::NotFound
                        | OperationError::Unavailable
                ) {
                    return operation_error(error);
                }
                self.index(
                    matches!(change, Change::Create).then_some(draft.as_slice()),
                    Some(&error),
                )
                .await
            }
        }
    }
}

fn attributes(draft: &[(String, String)]) -> Value {
    json!({"email":field(draft,"email"),"membership_role":field(draft,"membership_role"),"relationship_type":field(draft,"relationship_type"),"access_level":field(draft,"access_level"),"dependent_ids":draft.iter().filter(|(key,_)|key=="dependent_ids[]").map(|(_,value)|value).collect::<Vec<_>>()})
}
