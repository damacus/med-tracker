use super::*;
use administration::member_browser::{self, MemberQuery, MembershipId};
use administration::memberships::{self, LifecycleAction};
use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Suspend,
    Revoke,
}
impl Action {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "suspend" => Some(Self::Suspend),
            "revoke" => Some(Self::Revoke),
            _ => None,
        }
    }
    fn value(self) -> &'static str {
        match self {
            Self::Suspend => "suspend",
            Self::Revoke => "revoke",
        }
    }
    fn canonical(self) -> LifecycleAction {
        match self {
            Self::Suspend => LifecycleAction::Suspend,
            Self::Revoke => LifecycleAction::Revoke,
        }
    }
}
struct Submission {
    ids: Vec<MembershipId>,
    action: Action,
    context: HashMap<String, String>,
    query: MemberQuery,
    language: &'static str,
}
impl Submission {
    fn parse(
        fields: Vec<(String, String)>,
        confirmed: bool,
    ) -> std::result::Result<Self, OperationError> {
        let mut ids = Vec::new();
        let mut context = HashMap::new();
        for (key, value) in fields {
            if key == "membership_ids" {
                let id = value
                    .parse::<i64>()
                    .ok()
                    .filter(|id| *id > 0)
                    .and_then(|id| MembershipId::new(id).ok())
                    .ok_or_else(invalid)?;
                if ids.len() == 25 || ids.contains(&id) {
                    return Err(invalid());
                }
                ids.push(id);
            } else if ![
                "authenticity_token",
                "action",
                "confirmation",
                "q",
                "role",
                "status",
                "sort",
                "direction",
                "page",
                "lang",
            ]
            .contains(&key.as_str())
                || context.insert(key, value).is_some()
            {
                return Err(invalid());
            }
        }
        if ids.is_empty() {
            return Err(invalid());
        }
        let action = Action::parse(forms::field(&context, "action")).ok_or_else(invalid)?;
        if confirmed && forms::field(&context, "confirmation") != action.value() {
            return Err(invalid());
        }
        let language = match context.get("lang") {
            Some(value) => supported_language(value).ok_or_else(invalid)?,
            None => "en",
        };
        let query = MemberQuery::from_form(&context)?;
        Ok(Self {
            ids,
            action,
            context,
            query,
            language,
        })
    }
}
fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{"base":["Select between 1 and 25 distinct memberships and a valid action."]}}),
    }
}
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Changed,
    StateChanged,
    OwnerProtected,
    UnavailableMember,
    Rejected,
    AuthorityLost,
    NotAttempted,
    Unconfirmed,
}
#[derive(Serialize)]
struct Target {
    id: MembershipId,
    name: Option<String>,
    outcome: Option<Outcome>,
}
struct Snapshot {
    targets: Vec<Target>,
    actor_membership: i64,
    account: i64,
    household_name: String,
}

struct BulkPage<'a> {
    ctx: &'a AppContext,
    slug: &'a str,
    session: &'a Session<SessionPgPool>,
    request: String,
    token: &'a CsrfToken,
    view: &'a TeraView,
}

pub(super) async fn preview(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(fields): AxumForm<Vec<(String, String)>>,
) -> Response {
    run(
        BulkPage {
            ctx: &ctx,
            slug: &slug,
            session: &session,
            request: request_id(request),
            token: &token,
            view: &view,
        },
        fields,
        false,
    )
    .await
}
pub(super) async fn apply(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(fields): AxumForm<Vec<(String, String)>>,
) -> Response {
    run(
        BulkPage {
            ctx: &ctx,
            slug: &slug,
            session: &session,
            request: request_id(request),
            token: &token,
            view: &view,
        },
        fields,
        true,
    )
    .await
}
async fn snapshot(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    slug: &str,
    request: &str,
    submission: &Submission,
) -> std::result::Result<Snapshot, OperationError> {
    let (principal, tenant) = begin(ctx, session, slug, request)
        .await
        .map_err(authentication_operation)?;
    administration::authorize(&tenant).await?;
    let members = member_browser::selected(&tenant, &submission.ids).await?;
    let household_name = administration::overview::household_name(&tenant).await?;
    let actor_membership = tenant.membership().id;
    let account = principal.account_id();
    let targets = submission
        .ids
        .iter()
        .map(|id| Target {
            id: *id,
            name: members
                .iter()
                .find(|member| member.id == *id)
                .map(|member| member.name.clone()),
            outcome: None,
        })
        .collect();
    tenant.commit().await?;
    Ok(Snapshot {
        targets,
        actor_membership,
        account,
        household_name,
    })
}
fn authentication_operation(error: AuthenticationError) -> OperationError {
    match error {
        AuthenticationError::Unavailable => OperationError::Unavailable,
        _ => OperationError::Forbidden,
    }
}
async fn run(page: BulkPage<'_>, fields: Vec<(String, String)>, confirmed: bool) -> Response {
    let BulkPage {
        ctx,
        slug,
        session,
        request,
        token,
        view,
    } = page;
    let language = fields
        .iter()
        .find(|(key, _)| key == "lang")
        .and_then(|(_, value)| supported_language(value))
        .unwrap_or("en");
    let submission = match Submission::parse(fields, confirmed) {
        Ok(value) => value,
        Err(_) => return invalid_selection(view, token, language),
    };
    if token
        .verify(forms::field(&submission.context, "authenticity_token"))
        .is_err()
    {
        return presentation_error(view, submission.language, OperationError::Forbidden);
    }
    let mut snapshot = match snapshot(ctx, session, slug, &request, &submission).await {
        Ok(value) => value,
        Err(error) => return presentation_error(view, submission.language, error),
    };
    let mut can_manage = true;
    if confirmed {
        snapshot.targets.sort_by_key(|target| {
            (
                target.id.get() == snapshot.actor_membership,
                target.id.get(),
            )
        });
        let mut stopped = false;
        for target in &mut snapshot.targets {
            if stopped {
                target.outcome = Some(Outcome::NotAttempted);
                continue;
            }
            let (principal, tenant) = match begin(ctx, session, slug, &request).await {
                Ok(value) => value,
                Err(error) => {
                    let reason = match error {
                        AuthenticationError::Unavailable => Outcome::Unconfirmed,
                        _ => Outcome::AuthorityLost,
                    };
                    target.outcome = Some(reason);
                    stopped = true;
                    can_manage = matches!(reason, Outcome::Unconfirmed);
                    continue;
                }
            };
            if principal.account_id() != snapshot.account {
                target.outcome = Some(match tenant.rollback().await {
                    Ok(()) => Outcome::AuthorityLost,
                    Err(_) => Outcome::Unconfirmed,
                });
                stopped = true;
                can_manage = false;
                continue;
            }
            let result = memberships::change_browser_status(
                &tenant,
                target.id,
                submission.action.canonical(),
                Some(principal.provenance()),
            )
            .await;
            match result {
                Ok(()) => match tenant.commit().await {
                    Ok(()) => target.outcome = Some(Outcome::Changed),
                    Err(_) => {
                        target.outcome = Some(Outcome::Unconfirmed);
                        stopped = true;
                    }
                },
                Err(OperationError::Validation { details }) => {
                    let reason = validation_outcome(&details);
                    match tenant.commit().await {
                        Ok(()) => target.outcome = Some(reason),
                        Err(_) => {
                            target.outcome = Some(Outcome::Unconfirmed);
                            stopped = true;
                        }
                    }
                }
                Err(error) => {
                    let forbidden = matches!(error, OperationError::Forbidden);
                    let reason = if forbidden {
                        Outcome::AuthorityLost
                    } else if matches!(error, OperationError::NotFound) {
                        Outcome::UnavailableMember
                    } else {
                        Outcome::Unconfirmed
                    };
                    target.outcome = Some(match tenant.rollback().await {
                        Ok(()) => reason,
                        Err(_) => {
                            stopped = true;
                            Outcome::Unconfirmed
                        }
                    });
                    if matches!(reason, Outcome::Unconfirmed) {
                        stopped = true;
                    }
                    if forbidden {
                        stopped = true;
                        can_manage = false;
                    }
                }
            }
        }
        if can_manage {
            can_manage = match begin(ctx, session, slug, &request).await {
                Ok((principal, tenant)) => {
                    let allowed = principal.account_id() == snapshot.account
                        && administration::authorize(&tenant).await.is_ok();
                    allowed && tenant.commit().await.is_ok()
                }
                Err(_) => false,
            };
        }
    }
    render(
        view,
        token,
        slug,
        &submission,
        &snapshot,
        confirmed,
        can_manage,
    )
}
fn validation_outcome(details: &Value) -> Outcome {
    let messages = details
        .get("errors")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|fields| fields.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(Value::as_str);
    for message in messages {
        match message {
            "Last active owner cannot be removed" | "Owner change is not authorized" => {
                return Outcome::OwnerProtected;
            }
            "Membership state has changed. Refresh the member list" => {
                return Outcome::StateChanged;
            }
            _ => {}
        }
    }
    Outcome::Rejected
}
fn render(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    submission: &Submission,
    snapshot: &Snapshot,
    confirmed: bool,
    can_manage: bool,
) -> Response {
    let Ok(labels) = crate::models::care::report_pdf::translations(submission.language) else {
        return unavailable();
    };
    let mut data = rendering::context(slug, None);
    data["labels"] = labels.clone();
    data["admin_labels"] = labels["administration"].clone();
    data["bulk_labels"] = labels["administration"]["bulk_members"].clone();
    data["member_labels"] = labels["administration"]["member_list"].clone();
    data["detail_labels"] = labels["administration"]["member_detail"].clone();
    data["lang"] = json!(submission.language);
    data["language_suffix"] = json!(language_suffix(
        submission.language,
        submission.context.contains_key("lang")
    ));
    data["title"] = labels["administration"]["bulk_members"]
        [if confirmed { "results" } else { "title" }]
    .clone();
    data["household_name"] = json!(snapshot.household_name);
    data["admin_section"] = json!("users");
    data["targets"] = json!(snapshot.targets);
    data["action"] = json!(submission.action);
    data["confirmed"] = json!(confirmed);
    data["can_manage"] = json!(can_manage);
    data["retry_available"] = json!(
        confirmed
            && can_manage
            && snapshot
                .targets
                .iter()
                .any(|target| !matches!(target.outcome, Some(Outcome::Changed)))
    );
    data["context"] = json!(submission.context);
    data["return_url"] = json!(format!(
        "/households/{slug}/admin/users{}",
        submission.query.suffix(
            submission
                .context
                .contains_key("lang")
                .then_some(submission.language)
        )
    ));
    rendering::render(view, token, "administration/bulk_members.html", data, None)
}

fn invalid_selection(view: &TeraView, token: &CsrfToken, language: &str) -> Response {
    let Ok(labels) = crate::models::care::report_pdf::translations(language) else {
        return unavailable();
    };
    let mut data = super::super::medications::rendering::appearance_context();
    data["labels"] = labels.clone();
    data["lang"] = json!(language);
    data["title"] = labels["administration"]["bulk_members"]["invalid_selection"].clone();
    data["bulk_labels"] = labels["administration"]["bulk_members"].clone();
    data["error_labels"] = labels["administration"]["errors"].clone();
    let response = rendering::render(view, token, "administration/bulk_invalid.html", data, None);
    (StatusCode::UNPROCESSABLE_ENTITY, response).into_response()
}
