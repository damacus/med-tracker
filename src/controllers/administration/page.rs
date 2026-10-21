use super::*;
use crate::models::care::administration::recovery::{
    self, RecoveryAction, RecoveryCommand, RecoveryGuardianId, RecoverySnapshot, RecoverySubjectId,
};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) enum Screen {
    Index,
    Settings,
    Users,
    Role(i64),
    Relationships,
    NewRelationship,
    Recovery,
    RecoveryDetail(i64),
}
#[derive(Clone, Copy)]
pub(super) enum Change {
    Settings,
    Role(i64),
    CreateRelationship,
    Relationship(i64, bool),
    Recovery(i64),
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
        self.language = match language {
            "cy" => "cy",
            "ga" => "ga",
            "pt" => "pt",
            "es" => "es",
            _ => "en",
        };
        self
    }

    pub(super) fn with_explicit_language(mut self, explicit: bool) -> Self {
        self.explicit_language = explicit;
        self
    }

    pub(super) async fn show(
        &self,
        screen: Screen,
        submitted: Option<&HashMap<String, String>>,
        error: Option<&OperationError>,
    ) -> Response {
        let (_, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id).await {
            Ok(value) => value,
            Err(error) => {
                return super::presentation_authentication_error(self.view, self.language, error);
            }
        };
        let result = self.data(&tenant, screen, submitted, error).await;
        let (template, data) = match result {
            Ok(value) => value,
            Err(error) => return super::presentation_error(self.view, self.language, error),
        };
        let response = rendering::render(self.view, self.token, template, data, error);
        if tenant.commit().await.is_err() {
            return super::presentation_error(
                self.view,
                self.language,
                OperationError::Unavailable,
            );
        }
        response
    }

    async fn data(
        &self,
        tenant: &TenantTransaction,
        screen: Screen,
        submitted: Option<&HashMap<String, String>>,
        error: Option<&OperationError>,
    ) -> std::result::Result<(&'static str, Value), OperationError> {
        administration::authorize(tenant).await?;
        let mut data = rendering::context(self.slug, error);
        let labels = crate::models::care::report_pdf::translations(self.language)
            .map_err(|_| OperationError::Unavailable)?;
        data["lang"] = json!(self.language);
        data["language_suffix"] = json!(super::language_suffix(
            self.language,
            self.explicit_language
        ));
        data["labels"] = labels.clone();
        data["admin_labels"] = labels["administration"].clone();
        data["overview_labels"] = labels["administration"]["overview"].clone();
        let household_name = administration::overview::household_name(tenant).await?;
        data["household_name"] = json!(household_name);
        let (title_key, section) = match screen {
            Screen::Index => ("overview", "overview"),
            Screen::Settings => ("settings", "settings"),
            Screen::Users | Screen::Role(_) => ("members", "users"),
            Screen::Relationships
            | Screen::NewRelationship
            | Screen::Recovery
            | Screen::RecoveryDetail(_) => ("relationships", "relationships"),
        };
        let section_title = labels["administration"]["nav"][title_key]
            .as_str()
            .ok_or(OperationError::Unavailable)?;
        data["title"] = json!(format!("{household_name} · {section_title}"));
        data["admin_section"] = json!(section);
        let template = match screen {
            Screen::Index => {
                let overview = administration::overview::read(tenant).await?;
                let heading = format!(
                    "{} {}",
                    overview.household_name,
                    labels["administration"]["overview"]["title"]
                        .as_str()
                        .ok_or(OperationError::Unavailable)?
                );
                data["household_name"] = json!(overview.household_name);
                data["active_members"] = json!(overview.active_members);
                data["people_count"] = json!(overview.people);
                data["pending_invitations"] = json!(overview.pending_invitations);
                data["title"] = json!(heading);
                let warnings = crate::models::care::care_warning::project(tenant, None).await?;
                let mut warnings = warnings.into_values().collect::<Vec<_>>();
                warnings.sort_by(|a, b| a["person_name"].as_str().cmp(&b["person_name"].as_str()));
                data["care_warning_count"] = json!(warnings.len());
                data["care_warnings"] = json!(warnings);
                data["care_warning_labels"] = labels["care_warning"].clone();
                let recent = administration::audit_logs::recent(tenant).await?;
                data["recent_activity"] = json!(super::audit_logs::summaries(
                    &recent,
                    &labels,
                    self.language
                )?);
                "administration/index.html"
            }
            Screen::Settings => {
                let current = administration::settings::read(tenant).await?;
                data["draft"] = submitted.map_or_else(
                    || json!({"name":current["data"]["name"]}),
                    |draft| json!(draft),
                );
                "administration/household.html"
            }
            Screen::Users => {
                data["members"] = json!(rendering::members(
                    &administration::memberships::list(tenant).await?
                )?);
                "administration/users.html"
            }
            Screen::Role(user_id) => {
                let member = self.member(tenant, user_id).await?;
                data["title"] = json!(format!(
                    "{} · Edit household role for {}",
                    household_name,
                    member["name"].as_str().unwrap_or("member")
                ));
                data["draft"] =
                    submitted.map_or_else(|| json!({"role":member["role"]}), |draft| json!(draft));
                data["member"] = member;
                "administration/role.html"
            }
            Screen::Relationships => {
                data["relationships"] =
                    administration::delegation::list(tenant).await?["data"].clone();
                data["unavailable_person"] =
                    crate::models::care::report_pdf::translations("en")
                        .map_err(|_| OperationError::Unavailable)?["admin"]["carer_relationships"]
                        ["index"]["unavailable_person"]
                        .clone();
                "administration/relationships.html"
            }
            Screen::NewRelationship => {
                data["title"] = json!(format!("{household_name} · New relationship"));
                let draft = submitted.cloned().unwrap_or_else(|| {
                    HashMap::from([
                        ("carer_id".into(), String::new()),
                        ("patient_id".into(), String::new()),
                        ("relationship_type".into(), String::new()),
                    ])
                });
                let mut options = administration::delegation::options(tenant).await?;
                rendering::selected(&mut options, &draft);
                data["carers"] = options["carers"].clone();
                data["patients"] = options["patients"].clone();
                data["draft"] = json!(draft);
                "administration/relationship_form.html"
            }
            Screen::Recovery => {
                let options = recovery::options(tenant).await?;
                data["subjects"] = json!(
                    options
                        .subjects
                        .into_iter()
                        .filter(|subject| !subject.has_effective_guardian)
                        .map(|subject| json!({"id":subject.id.get(),"name":subject.name}))
                        .collect::<Vec<_>>()
                );
                data["subjects_has_more"] = json!(options.subjects_has_more);
                "administration/recovery.html"
            }
            Screen::RecoveryDetail(id) => {
                let options = recovery::options(tenant).await?;
                let subject = options
                    .subjects
                    .into_iter()
                    .find(|subject| subject.id.get() == id && !subject.has_effective_guardian)
                    .ok_or(OperationError::NotFound)?;
                data["subject"] = json!({
                    "id":subject.id.get(),"name":subject.name,
                    "snapshot_token":subject.snapshot.token.to_string(),
                    "snapshot_revision":subject.snapshot.revision,
                    "link_token":subject.snapshot.link_token.to_string(),
                    "restorable":subject.restorable.map(|link| json!({
                        "declaration_id":link.declaration_id,"relationship_id":link.relationship_id,
                        "guardian_id":link.guardian_id.get()
                    }))
                });
                data["guardians"] = json!(
                    options
                        .guardians
                        .into_iter()
                        .map(|guardian| json!({"id":guardian.id.get(),"name":guardian.name}))
                        .collect::<Vec<_>>()
                );
                data["guardians_has_more"] = json!(options.guardians_has_more);
                data["draft"] = json!(submitted);
                "administration/recovery_detail.html"
            }
        };
        Ok((template, data))
    }

    async fn member(
        &self,
        tenant: &TenantTransaction,
        user_id: i64,
    ) -> std::result::Result<Value, OperationError> {
        rendering::members(&administration::memberships::list(tenant).await?)?
            .into_iter()
            .find(|member| member["user_id"].as_i64() == Some(user_id))
            .ok_or(OperationError::NotFound)
    }

    pub(super) async fn save(&self, change: Change, draft: HashMap<String, String>) -> Response {
        if self
            .token
            .verify(forms::field(&draft, "authenticity_token"))
            .is_err()
        {
            return super::presentation_error(self.view, self.language, OperationError::Forbidden);
        }
        let (principal, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                return super::presentation_authentication_error(self.view, self.language, error);
            }
        };
        let provenance = Some(principal.provenance());
        let result = async {
            match change {
                Change::Settings => {
                    administration::settings::update(
                        &tenant,
                        json!({"name":forms::field(&draft,"name")}),
                        provenance,
                    )
                    .await?;
                    Ok(("household/edit", Screen::Settings))
                }
                Change::Role(id) => {
                    let member = self.member(&tenant, id).await?;
                    let membership_id = member["id"].as_i64().ok_or(OperationError::Unavailable)?;
                    administration::memberships::change(
                        &tenant,
                        membership_id,
                        json!({"role":forms::field(&draft,"role")}),
                        false,
                        provenance,
                    )
                    .await?;
                    Ok(("users", Screen::Role(id)))
                }
                Change::CreateRelationship => {
                    let carer = positive_id(&draft, "carer_id")?;
                    let patient = positive_id(&draft, "patient_id")?;
                    administration::delegation::assign(
                        &tenant,
                        carer,
                        patient,
                        forms::field(&draft, "relationship_type"),
                        provenance,
                    )
                    .await?;
                    Ok(("carer_relationships", Screen::NewRelationship))
                }
                Change::Relationship(id, active) => {
                    if active {
                        administration::delegation::reactivate(&tenant, id, provenance).await?;
                    } else {
                        administration::delegation::deactivate(&tenant, id, provenance).await?;
                    }
                    Ok(("carer_relationships", Screen::Relationships))
                }
                Change::Recovery(id) => {
                    if forms::field(&draft, "confirm_recovery") != "true" {
                        return Err(validation("confirm_recovery", "must be confirmed"));
                    }
                    let subject = RecoverySubjectId::parse(id)?;
                    let action = match forms::field(&draft, "action") {
                        "restore" => RecoveryAction::Restore {
                            declaration_id: positive_id(&draft, "declaration_id")?,
                            relationship_id: positive_id(&draft, "relationship_id")?,
                        },
                        "appoint" => RecoveryAction::Appoint,
                        _ => return Err(validation("action", "is invalid")),
                    };
                    let guardian = RecoveryGuardianId::parse(positive_id(&draft, "guardian_id")?)?;
                    let expected = RecoverySnapshot {
                        token: uuid_field(&draft, "snapshot_token")?,
                        revision: nonnegative_id(&draft, "snapshot_revision")?,
                        link_token: uuid_field(&draft, "link_token")?,
                    };
                    recovery::execute(
                        &tenant,
                        RecoveryCommand {
                            subject,
                            guardian,
                            expected,
                            action,
                            reason: forms::field(&draft, "reason").to_owned(),
                        },
                    )
                    .await?;
                    Ok(("carer_relationships", Screen::RecoveryDetail(id)))
                }
            }
        }
        .await;
        match result {
            Ok((path, _)) => {
                if tenant.commit().await.is_err() {
                    return super::presentation_error(
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
                                "/households/{}/admin/{path}{}",
                                self.slug,
                                super::language_suffix(self.language, self.explicit_language)
                            ),
                        ),
                        (header::CACHE_CONTROL, "no-store".into()),
                    ],
                )
                    .into_response()
            }
            Err(error) => {
                let finish = if matches!(change, Change::Role(_))
                    && matches!(error, OperationError::Validation { .. })
                {
                    tenant.commit().await
                } else {
                    tenant.rollback().await
                };
                if finish.is_err() {
                    return super::presentation_error(
                        self.view,
                        self.language,
                        OperationError::Unavailable,
                    );
                }
                let screen = match change {
                    Change::Settings => Screen::Settings,
                    Change::Role(id) => Screen::Role(id),
                    Change::CreateRelationship => Screen::NewRelationship,
                    Change::Relationship(_, _) => Screen::Relationships,
                    Change::Recovery(id) => {
                        if matches!(&error, OperationError::Conflict { code, .. } if code == "stale_guardian_recovery" || code == "guardian_already_effective")
                        {
                            Screen::Recovery
                        } else {
                            Screen::RecoveryDetail(id)
                        }
                    }
                };
                self.show(screen, Some(&draft), Some(&error)).await
            }
        }
    }
}

fn validation(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}

fn uuid_field(draft: &HashMap<String, String>, field: &str) -> Result<Uuid, OperationError> {
    Uuid::parse_str(forms::field(draft, field)).map_err(|_| validation(field, "is invalid"))
}

fn nonnegative_id(draft: &HashMap<String, String>, field: &str) -> Result<i64, OperationError> {
    forms::field(draft, field)
        .parse::<i64>()
        .ok()
        .filter(|id| *id >= 0)
        .ok_or_else(|| validation(field, "is invalid"))
}

fn positive_id(
    draft: &HashMap<String, String>,
    field: &str,
) -> std::result::Result<i64, OperationError> {
    forms::field(draft, field)
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| OperationError::Validation {
            details: json!({"errors":{field:["is invalid"]}}),
        })
}
