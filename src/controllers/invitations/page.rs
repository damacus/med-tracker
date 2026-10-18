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
    language: &'static str,
    explicit_language: bool,
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
            language: "en",
            explicit_language: false,
        }
    }

    pub(super) fn with_language(mut self, language: &str) -> Self {
        self.language = crate::models::care::report_pdf::locale(language);
        self
    }

    pub(super) fn with_explicit_language(mut self, explicit: bool) -> Self {
        self.explicit_language = explicit;
        self
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
            let household_name =
                crate::models::care::administration::overview::household_name(&tenant).await?;
            Ok::<_, OperationError>(rendering::data(
                self.slug,
                &household_name,
                &records,
                submitted,
                error,
            ))
        }
        .await;
        let mut data = match result {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
        let labels = match crate::models::care::report_pdf::translations(self.language) {
            Ok(labels) => labels,
            Err(_) => return unavailable(),
        };
        data["lang"] = json!(self.language);
        data["language_suffix"] = json!(super::super::administration::language_suffix(
            self.language,
            self.explicit_language
        ));
        data["admin_labels"] = labels["administration"].clone();
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
                            format!(
                                "/households/{}/admin/invitations{}",
                                self.slug,
                                super::super::administration::language_suffix(
                                    self.language,
                                    self.explicit_language
                                )
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
    let mut attributes =
        json!({"email":field(draft,"email"),"membership_role":field(draft,"membership_role")});
    for (name, value) in draft.iter().filter(|(name, _)| {
        matches!(
            name.as_str(),
            "dependent_ids" | "dependent_ids[]" | "relationship_type" | "access_level"
        )
    }) {
        attributes[name] = json!(value);
    }
    attributes
}
