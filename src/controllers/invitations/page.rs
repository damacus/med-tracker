use super::*;

#[derive(Clone, Copy)]
pub(super) enum Change {
    Create,
    Resend(i64),
    Cancel(i64),
}

#[derive(Clone, Copy)]
pub(super) enum Notice {
    Created,
    Resent,
    Cancelled,
}

impl Notice {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "created" => Some(Self::Created),
            "resent" => Some(Self::Resent),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Resent => "resent",
            Self::Cancelled => "cancelled",
        }
    }
}

impl Change {
    fn notice(self) -> Notice {
        match self {
            Self::Create => Notice::Created,
            Self::Resend(_) => Notice::Resent,
            Self::Cancel(_) => Notice::Cancelled,
        }
    }
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
    cursor: Option<invitations::browser::BrowseCursor>,
    notice: Option<Notice>,
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
            cursor: None,
            notice: None,
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

    pub(super) fn with_cursor(
        mut self,
        cursor: Option<invitations::browser::BrowseCursor>,
    ) -> Self {
        self.cursor = cursor;
        self
    }

    pub(super) fn with_notice(mut self, notice: Option<Notice>) -> Self {
        self.notice = notice;
        self
    }

    pub(super) async fn index(
        &self,
        submitted: Option<&[(String, String)]>,
        error: Option<&OperationError>,
    ) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => {
                return super::super::administration::presentation_authentication_error(
                    self.view,
                    self.language,
                    error,
                );
            }
        };
        let labels = match crate::models::care::report_pdf::translations(self.language) {
            Ok(labels) => labels,
            Err(_) => {
                return super::super::administration::presentation_error(
                    self.view,
                    self.language,
                    OperationError::Unavailable,
                );
            }
        };
        let result = async {
            let records = invitations::browser::browse(&tenant, self.cursor).await?;
            let household_name =
                crate::models::care::administration::overview::household_name(&tenant).await?;
            let mut data = rendering::data(
                self.slug,
                &household_name,
                &records,
                submitted,
                error,
                &labels,
            );
            data["notice"] = json!(self.notice.map(|notice| {
                labels["admin"]["invitations"][notice.key()]
                    .as_str()
                    .unwrap_or_default()
            }));
            data["next_url"] = json!(records.next_cursor.as_deref().map(|cursor| {
                let mut query = url::form_urlencoded::Serializer::new(String::new());
                query.append_pair("cursor", cursor);
                if self.explicit_language || self.language != "en" {
                    query.append_pair("lang", self.language);
                }
                format!(
                    "/households/{}/admin/invitations?{}",
                    self.slug,
                    query.finish()
                )
            }));
            Ok::<_, OperationError>(data)
        }
        .await;
        let mut data = match result {
            Ok(value) => value,
            Err(error) => {
                return super::super::administration::presentation_error(
                    self.view,
                    self.language,
                    error,
                );
            }
        };
        data["lang"] = json!(self.language);
        data["language_suffix"] = json!(super::super::administration::language_suffix(
            self.language,
            self.explicit_language
        ));
        data["labels"] = labels.clone();
        data["admin_labels"] = labels["administration"].clone();
        data["invite_labels"] = labels["admin"]["invitations"].clone();
        data["role_labels"] = labels["admin"]["labels"]["membership_roles"].clone();
        let response = rendering::render(self.view, self.token, data, error);
        if tenant.commit().await.is_err() {
            return super::super::administration::presentation_error(
                self.view,
                self.language,
                OperationError::Unavailable,
            );
        }
        response
    }

    pub(super) async fn save(&self, change: Change, draft: Vec<(String, String)>) -> Response {
        if self
            .token
            .verify(field(&draft, "authenticity_token"))
            .is_err()
        {
            return super::super::administration::presentation_error(
                self.view,
                self.language,
                OperationError::Forbidden,
            );
        }
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => {
                    return super::super::administration::presentation_authentication_error(
                        self.view,
                        self.language,
                        error,
                    );
                }
            };
        let target = match url::Url::parse(&self.ctx.config.server.full_url())
            .and_then(|url| url.join("/invitations/accept"))
        {
            Ok(url) => url,
            Err(_) => {
                return super::super::administration::presentation_error(
                    self.view,
                    self.language,
                    OperationError::Unavailable,
                );
            }
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
                    return super::super::administration::presentation_error(
                        self.view,
                        self.language,
                        OperationError::Unavailable,
                    );
                }
                (
                    StatusCode::SEE_OTHER,
                    [
                        (
                            header::LOCATION,
                            format!(
                                "/households/{}/admin/invitations{}notice={}",
                                self.slug,
                                if self.explicit_language || self.language != "en" {
                                    format!("?lang={}&", self.language)
                                } else {
                                    "?".into()
                                },
                                change.notice().key(),
                            ),
                        ),
                        (header::CACHE_CONTROL, "no-store".into()),
                    ],
                )
                    .into_response()
            }
            Err(error) => {
                if tenant.rollback().await.is_err() {
                    return super::super::administration::presentation_error(
                        self.view,
                        self.language,
                        OperationError::Unavailable,
                    );
                }
                if matches!(
                    error,
                    OperationError::Forbidden
                        | OperationError::NotFound
                        | OperationError::Unavailable
                ) {
                    return super::super::administration::presentation_error(
                        self.view,
                        self.language,
                        error,
                    );
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
