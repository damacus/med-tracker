use super::*;
use crate::controllers::shell::{ShellView as TeraView, ShellViewEngine as ViewEngine};
use crate::models::{
    access::{self, Actor},
    identity::{browser, resource::AuthenticationError},
    ids::AccountId,
    profile::self_account,
};
use axum::extract::{Form, Query};
use sea_orm::DatabaseTransaction;
use serde::Deserialize;

#[derive(Default, Deserialize)]
struct ProfileQuery {
    household: Option<String>,
}

mod avatar;
pub(crate) mod history;
pub(crate) mod person_schedules;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/profile")
        .add("/", get(show).post(save))
        .add("/history", get(history::index))
        .add("/schedules", get(person_schedules::index))
        .add("/schedules/{id}", post(person_schedules::save))
        .add(
            "/avatar",
            get(avatar::show)
                .post(avatar::upload)
                .layer(axum::extract::DefaultBodyLimit::max(6 * 1024 * 1024)),
        )
        .add("/avatar/remove", post(avatar::remove))
}

async fn begin(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    write: bool,
) -> std::result::Result<(browser::BrowserPrincipal, DatabaseTransaction), AuthenticationError> {
    let principal = browser::authenticate(&ctx.db, session, ctx.config.settings.as_ref()).await?;
    let transaction = principal.account_transaction(&ctx.db, write).await?;
    Ok((principal, transaction))
}

pub(super) struct AccountPage<'a> {
    pub(super) ctx: &'a AppContext,
    pub(super) session: &'a Session<SessionPgPool>,
    pub(super) headers: &'a HeaderMap,
    pub(super) token: &'a CsrfToken,
    pub(super) view: &'a TeraView,
    pub(super) request_id: String,
    pub(super) household: Option<&'a str>,
}

async fn show(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<ProfileQuery>,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    if view
        .profile_presentation()
        .is_some_and(|value| !value.complete)
    {
        return super::dialogs::open(super::dialogs::Open {
            ctx,
            session,
            headers,
            token,
            view,
            request,
        })
        .await;
    }
    AccountPage {
        ctx: &ctx,
        session: &session,
        headers: &headers,
        token: &token,
        view: &view,
        request_id: request_id(request),
        household: query.household.as_deref(),
    }
    .render(None)
    .await
}

fn changes(
    draft: &HashMap<String, String>,
) -> std::result::Result<self_account::Changes, OperationError> {
    let parsed = super::parse_preferences(draft)?;
    self_account::Changes::parse(
        None,
        parsed.time_zone.as_deref(),
        parsed.gravatar_enabled,
        parsed.mobile_shortcuts.as_deref(),
    )
}

async fn save(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    (Query(query), request): (Query<ProfileQuery>, Option<Extension<LocoRequestId>>),
    Form(draft): Form<HashMap<String, String>>,
) -> Response {
    if token
        .verify(draft.get("authenticity_token").map_or("", String::as_str))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let page = AccountPage {
        ctx: &ctx,
        session: &session,
        headers: &headers,
        token: &token,
        view: &view,
        request_id: request_id(request),
        household: query.household.as_deref(),
    };
    let (principal, transaction) = match begin(&ctx, &session, true).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = if draft
        .get("setting")
        .is_some_and(|setting| setting == "identity")
    {
        update_identity(&transaction, &principal, &page.request_id, &draft).await
    } else {
        match changes(&draft) {
            Ok(changes) => {
                self_account::update(
                    &transaction,
                    AccountId::from_database(principal.account_id()),
                    &page.request_id,
                    changes,
                )
                .await
            }
            Err(error) => Err(error),
        }
    };
    match result {
        Ok(()) => match transaction.commit().await {
            Ok(()) => redirect(
                page.household,
                match draft.get("setting").map(String::as_str) {
                    Some("time_zone" | "shortcuts") => "look-and-feel",
                    _ => "profile",
                },
            ),
            Err(error) => operation_error(error.into()),
        },
        Err(error) => {
            if let Err(error) = transaction.rollback().await {
                return operation_error(error.into());
            }
            if matches!(error, OperationError::Validation { .. }) {
                page.render(Some(draft)).await
            } else {
                operation_error(error)
            }
        }
    }
}

async fn update_identity(
    transaction: &DatabaseTransaction,
    principal: &browser::BrowserPrincipal,
    request_id: &str,
    draft: &HashMap<String, String>,
) -> std::result::Result<(), OperationError> {
    if draft.keys().any(|key| {
        ![
            "authenticity_token",
            "setting",
            "full_name",
            "date_of_birth",
            "use_display_name",
            "display_name",
        ]
        .contains(&key.as_str())
    }) {
        return Err(profile::validation(
            "profile",
            "contains an unsupported field",
        ));
    }
    let name = draft.get("full_name").map_or("", String::as_str);
    let alias = draft.get("display_name").map(String::as_str);
    let account_name = match draft.get("use_display_name").map(String::as_str) {
        Some("true") => alias.unwrap_or_default(),
        None => name,
        Some(_) => return Err(profile::validation("display_name", "is invalid")),
    };
    let identity = self_account::Changes::parse(Some(account_name), None, None, None)?
        .with_display_name(draft.contains_key("use_display_name"), alias)?;
    let clinical = crate::models::care::people::SelfClinicalChanges::parse(
        name,
        draft.get("date_of_birth").map_or("", String::as_str),
    )?;
    let account = AccountId::from_database(principal.account_id());
    crate::models::care::people::update_self(
        transaction,
        account,
        clinical,
        principal.time_zone(),
        request_id,
        principal.provenance(),
    )
    .await?;
    self_account::update(transaction, account, request_id, identity).await
}

fn redirect(household: Option<&str>, tab: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, super::profile_url(household, tab)),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
    )
        .into_response()
}

impl AccountPage<'_> {
    pub(super) async fn render(&self, draft: Option<HashMap<String, String>>) -> Response {
        let (principal, transaction) = match begin(self.ctx, self.session, false).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let account_id = AccountId::from_database(principal.account_id());
        let snapshot =
            match self_account::read(&transaction, account_id, self.ctx.config.settings.as_ref())
                .await
            {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
        let person = match access::person_queries::account_person(Actor {
            account_id: account_id.get(),
        })
        .one(&transaction)
        .await
        {
            Ok(Some(value)) => value,
            Ok(None) => return operation_error(OperationError::Unauthenticated),
            Err(error) => return operation_error(error.into()),
        };
        if let Err(error) = transaction.commit().await {
            return operation_error(error.into());
        }
        let Some(service) = self
            .ctx
            .shared_store
            .get::<crate::models::identity::better_auth::IdentityService>()
        else {
            return unavailable();
        };
        let security = match crate::models::identity::better_auth::security::overview(
            &service,
            &crate::controllers::identity_onboarding::identity_request(self.headers),
            self.request_id.clone(),
        )
        .await
        {
            Ok(value) if value.account_id == account_id => value,
            Ok(_) => return operation_error(OperationError::Forbidden),
            Err(_) => return unavailable(),
        };
        let (_, transaction) = match begin(self.ctx, self.session, false).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        match access::person_queries::account_person(Actor {
            account_id: account_id.get(),
        })
        .one(&transaction)
        .await
        {
            Ok(Some(current)) if current.id == person.id => {}
            Ok(_) => return operation_error(OperationError::Forbidden),
            Err(error) => return operation_error(error.into()),
        }
        if let Err(error) = transaction.commit().await {
            return operation_error(error.into());
        }
        let locale = report_pdf::locale(
            self.headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("en"),
        );
        let labels = match report_pdf::translations(locale) {
            Ok(value) => value,
            Err(_) => return unavailable(),
        };
        let labels = crate::views::profile_security::with_provider_name(
            labels,
            crate::models::identity::better_auth::oidc::provider_name(
                self.ctx.config.settings.as_ref(),
            ),
        );
        let Ok(authenticity_token) = self.token.authenticity_token() else {
            return unavailable();
        };
        let mut data = crate::controllers::medications::rendering::appearance_context();
        if let Some(palettes) = data["palettes"].as_array_mut() {
            for palette in palettes {
                let key = palette["id"].as_str().unwrap_or_default().replace('-', "_");
                palette["label"] = labels["profiles"]["theme_picker"]["themes"][&key].clone();
            }
        }
        let households = match principal.households(&self.ctx.db).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let active = if self.household.is_none() {
            match principal.active_household(&self.ctx.db).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            }
        } else {
            None
        };
        let selected = self
            .household
            .or_else(|| active.as_ref().map(|household| household.slug.as_str()));
        let scope = match selected {
            Some(slug) => match households.iter().find(|household| household.slug == slug) {
                Some(value) => Some(value),
                None if self.household.is_some() => {
                    return operation_error(OperationError::NotFound);
                }
                None => None,
            },
            None if households.len() == 1 => households.first(),
            None => None,
        };
        let slug = scope.map_or("", |household| household.slug.as_str());
        let can_manage = if scope.is_some() {
            let tenant = match principal
                .begin_household_slug(
                    &self.ctx.db,
                    slug,
                    self.request_id.clone(),
                    self.ctx.config.settings.as_ref(),
                )
                .await
            {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
            let notifications = match super::notifications::context(
                &tenant,
                principal.account_id(),
                &labels,
                draft.as_ref(),
            )
            .await
            {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            let can_manage = match crate::models::care::administration::can_manage(&tenant).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            if let Err(error) = tenant.commit().await {
                return operation_error(error);
            }
            data["notifications"] = notifications;
            can_manage
        } else {
            true
        };
        data["profile_households"] = json!(
            households
                .iter()
                .map(|household| { json!({"slug": household.slug, "name": household.name}) })
                .collect::<Vec<_>>()
        );
        data["profile_has_household"] = json!(scope.is_some());
        data["profile_household_name"] = json!(scope.map(|household| household.name.as_str()));
        let name = snapshot.name.as_deref().unwrap_or(&person.name);
        let selected_zone = draft
            .as_ref()
            .and_then(|fields| fields.get("time_zone"))
            .cloned()
            .unwrap_or_else(|| snapshot.time_zone.name().into());
        let mut zones = profile::supported_zones();
        if !zones.iter().any(|(_, zone)| zone == &selected_zone) {
            zones.push((selected_zone.clone(), selected_zone.clone()));
        }
        let setting = draft
            .as_ref()
            .and_then(|fields| fields.get("setting"))
            .map(String::as_str);
        data["title"] = labels["profiles"]["show"]["title"].clone();
        data["labels"] = labels.clone();
        data["lang"] = json!(locale);
        data["slug"] = json!(slug);
        let context_query = scope.map(|_| {
            url::form_urlencoded::Serializer::new(String::new())
                .append_pair("household", slug)
                .finish()
        });
        data["profile_query"] = json!(
            context_query
                .as_ref()
                .map_or_else(String::new, |query| format!("?{query}"))
        );
        data["profile_context_query"] = json!(context_query);
        data["name"] = json!(name);
        data["email"] = json!(snapshot.email);
        data["initials"] = json!(
            name.split_whitespace()
                .filter_map(|word| word.chars().next())
                .take(2)
                .flat_map(char::to_uppercase)
                .collect::<String>()
        );
        data["clinical_name"] = json!(
            draft
                .as_ref()
                .filter(|_| setting == Some("identity"))
                .and_then(|fields| fields.get(if setting == Some("identity") {
                    "full_name"
                } else {
                    "name"
                }))
                .map_or(person.name.as_str(), String::as_str)
        );
        data["clinical_date"] = json!(
            draft
                .as_ref()
                .filter(|_| setting == Some("identity"))
                .and_then(|fields| fields.get("date_of_birth"))
                .cloned()
                .unwrap_or_else(|| person
                    .date_of_birth
                    .map(|date| date.to_string())
                    .unwrap_or_default())
        );
        data["date_of_birth"] = json!(person.date_of_birth.map(|date| date.to_string()));
        data["age_present"] = json!(false);
        data["person_type"] = json!(person.person_type);
        data["has_capacity"] = json!(person.has_capacity);
        data["avatar_attached"] = json!(snapshot.avatar_attached);
        data["can_edit_profile"] = json!(true);
        data["gravatar_url"] = json!(if snapshot.gravatar_enabled && !snapshot.avatar_attached {
            profile::gravatar::image_url(&snapshot.email)
        } else {
            None
        });
        data["gravatar_enabled"] = json!(snapshot.gravatar_enabled);
        data["selected_gravatar"] = json!(if setting == Some("gravatar") {
            draft
                .as_ref()
                .and_then(|fields| fields.get("gravatar_enabled"))
                .is_some_and(|value| value == "true")
        } else {
            snapshot.gravatar_enabled
        });
        data["time_zone"] = json!(snapshot.time_zone.name());
        data["selected_zone"] = json!(selected_zone);
        data["zones"] = json!(
            zones
                .into_iter()
                .map(|(name, value)| json!({"name":name,"value":value}))
                .collect::<Vec<_>>()
        );
        data["timezone_error"] = json!(setting == Some("time_zone"));
        data["shortcuts_error"] = json!(setting == Some("shortcuts"));
        data["gravatar_error"] = json!(setting == Some("gravatar"));
        data["clinical_error"] = json!(setting == Some("identity"));
        data["notifications_error"] = json!(setting == Some("notifications"));
        data["use_display_name"] = json!(if setting == Some("identity") {
            draft
                .as_ref()
                .is_some_and(|fields| fields.contains_key("use_display_name"))
        } else {
            name != person.name || snapshot.display_name_enabled
        });
        data["account_name_draft"] = json!(
            draft
                .as_ref()
                .filter(|_| setting == Some("identity"))
                .and_then(|fields| fields.get("display_name").map(String::as_str))
                .or_else(|| if name == person.name {
                    snapshot.saved_display_name.as_deref()
                } else {
                    Some(name)
                })
                .unwrap_or_default()
        );
        data["avatar_error"] = draft
            .as_ref()
            .and_then(|fields| fields.get("avatar_error"))
            .map_or(Value::Null, |key| labels["profiles"]["avatar"][key].clone());
        data["shortcut_choices"] = json!(super::shortcut_choices(
            slug,
            &labels,
            can_manage || self.view.system_administrator()
        ));
        let stored = serde_json::to_value(snapshot.mobile_shortcuts).unwrap_or_default();
        data["shortcut_slots"] = json!((1..=3).map(|slot| {
            let selected = if setting == Some("shortcuts") { draft.as_ref().and_then(|fields| fields.get(&format!("shortcut_{slot}"))).cloned().unwrap_or_default() } else { stored[slot - 1].as_str().unwrap_or_default().into() };
            json!({"number":slot,"label":labels["profiles"]["mobile_shortcuts"]["slot"].as_str().unwrap_or("").replace("%{number}", &slot.to_string()),"selected":selected})
        }).collect::<Vec<_>>());
        data["sidebar_links"] = json!([]);
        data["authenticity_token"] = json!(authenticity_token);
        data["account_access"] =
            match profile::account_access::load(&self.ctx.db, security.account_id).await {
                Ok(value) => json!(value),
                Err(error) => return operation_error(error),
            };
        data["security"] = json!(crate::views::profile_security::context(
            &security,
            principal.time_zone(),
            &labels,
            chrono::Utc::now()
        ));
        data["system_info"] = json!({"development":self.ctx.environment == loco_rs::environment::Environment::Development,"version":env!("CARGO_PKG_VERSION"),"worktree":env!("MEDTRACKER_BUILD_WORKTREE"),"commit":env!("MEDTRACKER_BUILD_COMMIT")});
        match format::render().view(self.view, "profile/show.html", data) {
            Ok(response) => (
                if draft.is_some() {
                    StatusCode::UNPROCESSABLE_ENTITY
                } else {
                    StatusCode::OK
                },
                self.token.clone(),
                [(header::CACHE_CONTROL, "no-store")],
                response,
            )
                .into_response(),
            Err(_error) => {
                tracing::warn!(error.type = "render", "Account profile rendering failed");
                unavailable()
            }
        }
    }
}
