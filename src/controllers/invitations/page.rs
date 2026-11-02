use super::*;

#[derive(Clone, Copy)]
pub(super) enum Change {
    Create,
    CreateDependent,
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
            Self::Create | Self::CreateDependent => Notice::Created,
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
    filters: invitations::browser::BrowseFilters,
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
            filters: invitations::browser::BrowseFilters::default(),
        }
    }

    pub(super) fn with_query(mut self, query: IndexQuery) -> Self {
        self.language = query.language;
        self.explicit_language = query.explicit_language;
        self.cursor = query.cursor;
        self.notice = query.notice;
        self.filters = query.filters;
        self
    }

    fn query_pairs(&self, cursor: Option<String>) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if !self.filters.search.is_empty() {
            pairs.push(("q".into(), self.filters.search.clone()));
        }
        if let Some(role) = self.filters.role {
            pairs.push(("role".into(), role.key().into()));
        }
        if let Some(status) = self.filters.status {
            pairs.push(("status".into(), status.key().into()));
        }
        if let Some(cursor) = cursor {
            pairs.push(("cursor".into(), cursor));
        }
        if self.explicit_language || self.language != "en" {
            pairs.push(("lang".into(), self.language.into()));
        }
        pairs
    }

    fn url(&self, cursor: Option<String>, notice: Option<Notice>) -> String {
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.extend_pairs(self.query_pairs(cursor));
        if let Some(notice) = notice {
            query.append_pair("notice", notice.key());
        }
        let query = query.finish();
        format!(
            "/households/{}/admin/invitations{}{}",
            self.slug,
            if query.is_empty() { "" } else { "?" },
            query
        )
    }

    pub(super) async fn index(
        &self,
        submitted: Option<&[(String, String)]>,
        error: Option<&OperationError>,
    ) -> Response {
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
            let records = invitations::browser::browse(&tenant, self.cursor, &self.filters).await?;
            let household_name =
                crate::models::care::administration::overview::household_name(&tenant).await?;
            let mut data = rendering::data(
                self.slug,
                &household_name,
                &records,
                submitted,
                error,
                &labels,
                principal.time_zone(),
            )?;
            data["notice"] = json!(self.notice.map(|notice| {
                labels["admin"]["invitations"][notice.key()]
                    .as_str()
                    .unwrap_or_default()
            }));
            data["next_url"]=json!(records.next_cursor.as_ref().map(|cursor|self.url(Some(cursor.clone()),None)));
            data["context"]=json!(self.query_pairs(self.cursor.map(invitations::browser::BrowseCursor::encode)).into_iter().map(|(name,value)|json!({"name":name,"value":value})).collect::<Vec<_>>());
            data["filters"]=json!({"q":self.filters.search,"role":self.filters.role.map_or("",invitations::browser::InvitationRole::key),"status":self.filters.status.map_or("",invitations::browser::InvitationStatus::key)});
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
        if let Some(cursor) = self.cursor
            && let Err(error) = cursor.validate(tenant.scope().household_id, &self.filters)
        {
            let error = if tenant.rollback().await.is_err() {
                OperationError::Unavailable
            } else {
                error
            };
            return super::super::administration::presentation_error(
                self.view,
                self.language,
                error,
            );
        }
        let target = match crate::models::settings::public_origin(self.ctx)
            .and_then(|url| url.join("/invitations/accept").map_err(Into::into))
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
            Change::Create | Change::CreateDependent => invitations::create(
                &tenant,
                if matches!(change, Change::CreateDependent)
                    && !draft.iter().any(|(name, _)| {
                        matches!(
                            name.as_str(),
                            "person_id"
                                | "dependent_ids"
                                | "dependent_ids[]"
                                | "subject_person_id"
                                | "relationship_type"
                                | "access_level"
                        )
                    })
                {
                    invitations::InvitationInput::dependent(field(&draft, "email"))
                } else {
                    attributes(&draft)
                },
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
                            self.url(
                                self.cursor.map(invitations::browser::BrowseCursor::encode),
                                Some(change.notice()),
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
                    matches!(change, Change::Create | Change::CreateDependent)
                        .then_some(draft.as_slice()),
                    Some(&error),
                )
                .await
            }
        }
    }
}

fn attributes(draft: &[(String, String)]) -> invitations::InvitationInput {
    let care_authority_requested = draft.iter().any(|(name, _)| {
        matches!(
            name.as_str(),
            "person_id"
                | "subject_person_id"
                | "dependent_ids"
                | "dependent_ids[]"
                | "relationship_type"
                | "access_level"
        )
    });
    invitations::InvitationInput::from_form(
        field(draft, "email"),
        field(draft, "membership_role"),
        care_authority_requested,
    )
}
