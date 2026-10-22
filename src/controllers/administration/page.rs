use super::*;

#[derive(Clone, Copy)]
pub(super) enum Screen {
    Index,
    Settings,
    Users,
    Member(
        administration::member_browser::MembershipId,
        Option<administration::member_history::HistoryCursor>,
    ),
    Role(administration::member_browser::MemberLocator),
    Relationships,
    NewRelationship,
}
#[derive(Clone, Copy)]
pub(super) enum Change {
    Settings,
    Role(administration::member_browser::MemberLocator),
    Membership(
        administration::member_browser::MembershipId,
        MembershipAction,
    ),
    CreateRelationship,
    Relationship(i64, bool),
}
pub(super) use administration::memberships::LifecycleAction as MembershipAction;

pub(super) struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    request_id: String,
    language: &'static str,
    explicit_language: bool,
    member_query: administration::member_browser::MemberQuery,
}

impl<'a> Page<'a> {
    pub(super) fn view(&self) -> &TeraView {
        self.view
    }
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
            member_query: administration::member_browser::MemberQuery {
                page: 1,
                ..Default::default()
            },
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

    pub(super) fn with_member_query(
        mut self,
        query: administration::member_browser::MemberQuery,
    ) -> Self {
        self.member_query = query;
        self
    }

    pub(super) async fn show(
        &self,
        screen: Screen,
        submitted: Option<&HashMap<String, String>>,
        error: Option<&OperationError>,
    ) -> Response {
        let (principal, tenant) = match begin(self.ctx, self.session, self.slug, &self.request_id)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                return super::presentation_authentication_error(self.view, self.language, error);
            }
        };
        let result = self
            .data(&tenant, screen, submitted, error, principal.time_zone())
            .await;
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
        if response.status().is_success() && matches!(screen, Screen::Users) {
            super::bulk_members::clear_review(self.session);
        }
        response
    }

    async fn data(
        &self,
        tenant: &TenantTransaction,
        screen: Screen,
        submitted: Option<&HashMap<String, String>>,
        error: Option<&OperationError>,
        zone: chrono_tz::Tz,
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
        data["member_labels"] = labels["administration"]["member_list"].clone();
        data["bulk_labels"] = labels["administration"]["bulk_members"].clone();
        data["detail_labels"] = labels["administration"]["member_detail"].clone();
        let overview = if matches!(screen, Screen::Index) {
            Some(administration::overview::read(tenant).await?)
        } else {
            None
        };
        let household_name = match &overview {
            Some(overview) => overview.household_name.clone(),
            None => administration::overview::household_name(tenant).await?,
        };
        data["household_name"] = json!(household_name);
        let (title_key, section) = match screen {
            Screen::Index => ("overview", "overview"),
            Screen::Settings => ("settings", "settings"),
            Screen::Users | Screen::Role(_) | Screen::Member(_, _) => ("members", "users"),
            Screen::Relationships | Screen::NewRelationship => ("relationships", "relationships"),
        };
        let section_title = labels["administration"]["nav"][title_key]
            .as_str()
            .ok_or(OperationError::Unavailable)?;
        data["title"] = json!(format!("{household_name} · {section_title}"));
        data["admin_section"] = json!(section);
        let template = match screen {
            Screen::Index => {
                let overview = overview.ok_or(OperationError::Unavailable)?;
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
                data["expired_invitations"] = json!(overview.expired_invitations);
                data["active_schedules"] = json!(overview.active_schedules);
                data["recent_memberships"] = json!(overview.recent_memberships);
                data["recent_audit_events"] = json!(overview.recent_audit_events);
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
                let listing =
                    administration::member_browser::list(tenant, &self.member_query).await?;
                data["members"] = json!(listing.members);
                data["total"] = json!(listing.total);
                data["page"] = json!(listing.page);
                data["pages"] = json!(listing.pages);
                data["query"] = json!({"q":self.member_query.search,"role":self.member_query.role_value(),"status":self.member_query.status_value(),"sort":self.member_query.sort_value(),"direction":self.member_query.direction_value()});
                let lang = self.explicit_language.then_some(self.language);
                data["member_query_suffix"] = json!(self.member_query.suffix(lang));
                data["member_notice"] = json!(self.member_query.notice);
                data["previous_url"] = if listing.page > 1 {
                    json!(self.member_query.with_page(listing.page - 1).suffix(lang))
                } else {
                    json!("")
                };
                data["next_url"] = if listing.page < listing.pages {
                    json!(self.member_query.with_page(listing.page + 1).suffix(lang))
                } else {
                    json!("")
                };
                "administration/users.html"
            }
            Screen::Member(id, cursor) => {
                let member = self
                    .member(
                        tenant,
                        administration::member_browser::MemberLocator::Membership(id),
                    )
                    .await?;
                let history = administration::member_history::read(tenant, id, cursor).await?;
                let lang = self.explicit_language.then_some(self.language);
                let mut list_query = self.member_query.clone();
                list_query.return_to_member = false;
                let mut detail_query = list_query.clone();
                detail_query.return_to_member = true;
                data["member"] = json!(member);
                data["member_labels"] = labels["administration"]["member_list"].clone();
                data["detail_labels"] = labels["administration"]["member_detail"].clone();
                data["member_return_url"] = json!(format!(
                    "/households/{}/admin/users{}",
                    self.slug,
                    list_query.suffix(lang)
                ));
                data["member_query_suffix"] = json!(detail_query.suffix(lang));
                data["member_notice"] = json!(self.member_query.notice);
                data["query"] = json!({"q":self.member_query.search,"role":self.member_query.role_value(),"status":self.member_query.status_value(),"sort":self.member_query.sort_value(),"direction":self.member_query.direction_value()});
                data["page"] = json!(self.member_query.page);
                data["return_to_member"] = json!(true);
                let rows=history.events.iter().map(|event| {
                    use administration::audit_logs::AuditFieldLabel;
                    let mut changes=Vec::new();
                    for (kind,previous,next) in [("role",AuditFieldLabel::PreviousRole,AuditFieldLabel::NewRole),("status",AuditFieldLabel::PreviousStatus,AuditFieldLabel::NewStatus)] {
                        let value=|label:AuditFieldLabel| event.fields.iter().find(|field| std::mem::discriminant(&field.label)==std::mem::discriminant(&label)).map(|field|field.value.as_str());
                        let before=value(previous);let after=value(next);
                        if before != after {
                            changes.push(json!({"kind":kind,"previous":before,"next":after}));
                        }
                    }
                    let local=event.occurred_at.and_utc().with_timezone(&zone);
                    let date=crate::models::care::people::browser::birth_date(Some(&local.date_naive().to_string()),&labels).ok_or(OperationError::Unavailable)?;
                    Ok(json!({"summary":super::audit_logs::summary(&event.summary,&labels,self.language)?,
                        "kind":event.kind,"outcome":event.outcome,"actor_name":event.actor_name,
                        "occurred_at_iso":event.occurred_at.and_utc().to_rfc3339(),
                        "occurred_at_display":format!("{} {}",date,local.format("%H:%M %Z")),
                        "changes":changes}))
                }).collect::<std::result::Result<Vec<_>,OperationError>>()?;
                data["history"] = json!(rows);
                data["history_older_url"] = json!(history.next_cursor.map(|cursor| {
                    let suffix = list_query.suffix(lang);
                    let separator = if suffix.is_empty() { "?" } else { "&" };
                    format!(
                        "/households/{}/admin/memberships/{}{}{}history_cursor={}",
                        self.slug,
                        id.get(),
                        suffix,
                        separator,
                        cursor
                    )
                }));
                "administration/member.html"
            }
            Screen::Role(locator) => {
                let member = self.member(tenant, locator).await?;
                let member_json = json!(member);
                data["title"] = json!(format!(
                    "{} · {}",
                    household_name,
                    labels["administration"]["polish"]["edit_role"]
                        .as_str()
                        .ok_or(OperationError::Unavailable)?
                        .replace("%{name}", &member.name)
                ));
                data["draft"] =
                    submitted.map_or_else(|| json!({"role":member.role}), |draft| json!(draft));
                data["member"] = member_json;
                let lang = self.explicit_language.then_some(self.language);
                data["member_return_suffix"] = json!(self.member_query.suffix(lang));
                data["member_return_url"] = json!(if self.member_query.return_to_member {
                    format!(
                        "/households/{}/admin/memberships/{}{}",
                        self.slug,
                        member.id.get(),
                        self.member_query.suffix(lang)
                    )
                } else {
                    format!(
                        "/households/{}/admin/users{}",
                        self.slug,
                        self.member_query.suffix(lang)
                    )
                });
                data["return_to_member"] = json!(self.member_query.return_to_member);
                data["member_role_path"] = json!(match locator {
                    administration::member_browser::MemberLocator::Membership(_) =>
                        format!("memberships/{}", member.id.get()),
                    administration::member_browser::MemberLocator::LegacyUser(id) =>
                        format!("users/{}", id.get()),
                });
                data["member_query"] = json!({"q":self.member_query.search,"role":self.member_query.role_value(),"status":self.member_query.status_value(),"sort":self.member_query.sort_value(),"direction":self.member_query.direction_value(),"page":self.member_query.page});
                "administration/role.html"
            }
            Screen::Relationships => {
                data["relationships"] =
                    administration::delegation::list(tenant).await?["data"].clone();
                data["unavailable_person"] =
                    crate::models::care::report_pdf::translations(self.language)
                        .map_err(|_| OperationError::Unavailable)?["admin"]["carer_relationships"]
                        ["index"]["unavailable_person"]
                        .clone();
                "administration/relationships.html"
            }
            Screen::NewRelationship => {
                data["title"] = json!(format!(
                    "{household_name} · {}",
                    labels["administration"]["polish"]["new_relationship"]
                        .as_str()
                        .ok_or(OperationError::Unavailable)?
                ));
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
        };
        Ok((template, data))
    }

    async fn member(
        &self,
        tenant: &TenantTransaction,
        locator: administration::member_browser::MemberLocator,
    ) -> std::result::Result<administration::member_browser::MemberRow, OperationError> {
        administration::member_browser::find(tenant, locator).await
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
                    let role = administration::memberships::MembershipRole::parse(forms::field(
                        &draft, "role",
                    ))?;
                    administration::memberships::change_browser_role(
                        &tenant, member.id, role, provenance,
                    )
                    .await?;
                    Ok(("users", Screen::Role(id)))
                }
                Change::Membership(id, action) => {
                    if forms::field(&draft, "confirmation") != action.notice() {
                        return Err(OperationError::Validation {
                            details: json!({"errors":{"base":["Confirm the membership change"]}}),
                        });
                    }
                    administration::memberships::change_browser_status(
                        &tenant, id, action, provenance,
                    )
                    .await?;
                    Ok(("users", Screen::Users))
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
                            if self.member_query.return_to_member && matches!(change,Change::Membership(_,_)|Change::Role(administration::member_browser::MemberLocator::Membership(_))) {
                                let id=match change {
                                    Change::Membership(id,_)|Change::Role(administration::member_browser::MemberLocator::Membership(id))=>id,
                                    _=>return super::presentation_error(self.view,self.language,OperationError::Unavailable),
                                };
                                let suffix=self.member_query.suffix(self.explicit_language.then_some(self.language));
                                let notice=if let Change::Membership(_,action)=change {format!("{}notice={}",if suffix.is_empty(){"?"}else{"&"},action.notice())}else{String::new()};
                                format!("/households/{}/admin/memberships/{}{}{}",self.slug,id.get(),suffix,notice)
                            } else if let Change::Membership(_, action) = change {
                                let suffix = self
                                    .member_query
                                    .suffix(self.explicit_language.then_some(self.language));
                                let separator = if suffix.is_empty() { "?" } else { "&" };
                                format!(
                                    "/households/{}/admin/users{}{}notice={}",
                                    self.slug,
                                    suffix,
                                    separator,
                                    action.notice()
                                )
                            } else if matches!(change, Change::Role(_)) {
                                format!(
                                    "/households/{}/admin/users{}",
                                    self.slug,
                                    self.member_query
                                        .suffix(self.explicit_language.then_some(self.language))
                                )
                            } else {
                                format!(
                                    "/households/{}/admin/{path}{}",
                                    self.slug,
                                    super::language_suffix(self.language, self.explicit_language)
                                )
                            },
                        ),
                        (header::CACHE_CONTROL, "no-store".into()),
                    ],
                )
                    .into_response()
            }
            Err(error) => {
                let finish = if matches!(change, Change::Role(_) | Change::Membership(_, _))
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
                    Change::Membership(id, _) if self.member_query.return_to_member => {
                        Screen::Member(id, None)
                    }
                    Change::Membership(_, _) => Screen::Users,
                    Change::CreateRelationship => Screen::NewRelationship,
                    Change::Relationship(_, _) => Screen::Relationships,
                };
                if matches!(error, OperationError::Forbidden) {
                    return super::presentation_error(self.view, self.language, error);
                }
                self.show(screen, Some(&draft), Some(&error)).await
            }
        }
    }
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
