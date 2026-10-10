use super::*;
use crate::models::{
    access::{self, PersonAccess},
    care::{health_events::browser, report_pdf},
};
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub(super) struct Filters {
    locale: Option<String>,
    event_kind: Option<String>,
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(filters): Query<Filters>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        let (person, _) = people::read(&tenant, &id, principal.time_zone()).await?;
        let person_id = person["data"]["id"]
            .as_i64()
            .ok_or(OperationError::Unavailable)?;
        let events = browser::collection(&tenant, person_id).await?;
        let language = report_pdf::locale(filters.locale.as_deref().unwrap_or("en"));
        let labels = report_pdf::translations(language).map_err(|_| OperationError::Unavailable)?;
        let title = labels["health_events"]["index"]["title"]
            .as_str()
            .ok_or(OperationError::Unavailable)?
            .replace(
                "%{person}",
                person["data"]["name"].as_str().unwrap_or_default(),
            );
        let mut data = super::super::medications::rendering::appearance_context();
        data["slug"] = json!(slug);
        data["person"] = person["data"].clone();
        data["title"] = json!(title);
        data["lang"] = json!(language);
        data["labels"] = labels["health_events"].clone();
        data["back_label"] = labels["people"]["show"]["back"].clone();
        data["events"] = json!(events);
        data["can_record"] =
            json!(access::can_access_person(&tenant, person_id, PersonAccess::Record).await?);
        data["can_manage"] =
            json!(access::can_access_person(&tenant, person_id, PersonAccess::Manage).await?);
        Ok::<_, OperationError>(data)
    }
    .await;
    let data = match result {
        Ok(data) => data,
        Err(error) => return operation_error(error),
    };
    let response = rendering::render(&view, &token, "health_events/index.html", data, None);
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    response
}

struct FormPage<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    slug: &'a str,
    person_id: &'a str,
    request_id: String,
}

impl FormPage<'_> {
    async fn form(
        &self,
        id: Option<&str>,
        filters: &Filters,
        submitted: Option<&HashMap<String, String>>,
        error: Option<&OperationError>,
    ) -> Response {
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let result=async {
            let (person,_) = people::read(&tenant,self.person_id,principal.time_zone()).await?;
            let person_id=person["data"]["id"].as_i64().ok_or(OperationError::Unavailable)?;
            access::require_person_access(&tenant,person_id,if id.is_some(){PersonAccess::Manage}else{PersonAccess::Record}).await?;
            let language=report_pdf::locale(submitted.and_then(|draft|draft.get("locale").map(String::as_str)).or(filters.locale.as_deref()).unwrap_or("en"));
            let all_labels=report_pdf::translations(language).map_err(|_|OperationError::Unavailable)?;
            let labels=&all_labels["health_events"];
            let mut draft=HashMap::new();
            if let Some(id)=id {
                let (record,links,etag)=browser::event(&tenant,person_id,id).await?;
                draft.insert("etag".into(),etag);
                draft.insert("event_kind".into(),if record.event_kind==0{"illness"}else{"suspected_side_effect"}.into());
                draft.insert("title".into(),record.title);
                draft.insert("started_on".into(),record.started_on.to_string());
                draft.insert("ended_on".into(),record.ended_on.map(|date|date.to_string()).unwrap_or_default());
                draft.insert("ongoing".into(),if record.ended_on.is_none(){"1"}else{""}.into());
                draft.insert("severity".into(),record.severity.map(|value|match value{0=>"mild",1=>"moderate",_=>"severe"}.to_owned()).unwrap_or_default());
                draft.insert("notes".into(),record.notes.unwrap_or_default());
                draft.insert("action_taken".into(),record.action_taken.unwrap_or_default());
                draft.insert("medical_help_sought".into(),if record.medical_help_sought{"1"}else{""}.into());
                for link in links {
                    if let Some(id)=link.medication_id {draft.insert(format!("medication_{id}"),"1".into());}
                }
            } else {
                for field in ["etag","title","started_on","ended_on","severity","notes","action_taken","medical_help_sought"] {
                    draft.insert(field.into(),String::new());
                }
                draft.insert("ongoing".into(),"1".into());
                draft.insert("event_kind".into(),filters.event_kind.as_deref().filter(|kind|matches!(*kind,"illness"|"suspected_side_effect")).unwrap_or("illness").into());
            }
            if let Some(submitted)=submitted {
                draft=submitted.clone();
                for field in ["etag","event_kind","title","started_on","ended_on","ongoing","severity","notes","action_taken","medical_help_sought"] {
                    draft.entry(field.into()).or_default();
                }
            }
            let kind=draft.get("event_kind").map(String::as_str).unwrap_or("illness");
            let title=if id.is_none() {
                labels["actions"][if kind=="suspected_side_effect"{"record_suspected_side_effect"}else{"record_illness"}].as_str().unwrap_or_default().to_owned()
            } else {
                labels["form"]["edit_title"].as_str().unwrap_or_default().replace("%{kind}",&labels["kinds"][kind].as_str().unwrap_or_default().to_lowercase())
            };
            let options=browser::medication_options(&tenant,person_id).await?.into_iter().map(|row| {
                let key=format!("medication_{}",row.id);
                json!({"id":row.id,"name":row.name.or(row.friendly_name).unwrap_or_default(),"selected":draft.get(&key).is_some_and(|value|value=="1")})
            }).collect::<Vec<_>>();
            let end_error=draft.get("ongoing").is_none_or(|value|value!="1")
                && draft.get("started_on").and_then(|value|chrono::NaiveDate::parse_from_str(value,"%Y-%m-%d").ok())
                .zip(draft.get("ended_on").and_then(|value|chrono::NaiveDate::parse_from_str(value,"%Y-%m-%d").ok()))
                .is_some_and(|(start,end)|end<start);
            let error_message=error.map(|error|match error {
                OperationError::Conflict{..}=>labels["browser"]["stale_error"].clone(),
                OperationError::Unavailable=>labels["browser"]["unavailable_error"].clone(),
                OperationError::Validation{details} if details["errors"]["medication_ids"].is_array()=>labels["invalid_medication_link"].clone(),
                _ if end_error=>labels["browser"]["date_error"].clone(),
                _=>labels["browser"]["form_error"].clone(),
            });
            let mut data=super::super::medications::rendering::appearance_context();
            data["slug"]=json!(self.slug);
            data["person"]=person["data"].clone();
            data["title"]=json!(title);
            data["lang"]=json!(language);
            data["labels"]=labels.clone();
            data["draft"]=json!(draft);
            data["medications"]=json!(options);
            data["side_effect"]=json!(kind=="suspected_side_effect");
            data["error"]=json!(error_message);
            data["end_error"]=json!(end_error && error.is_some());
            data["editing"]=json!(id.is_some());
            data["latest_url"]=json!(id.map(|id|format!("/households/{}/people/{}/health_events/{id}/edit?locale={language}",self.slug,person_id)));
            data["action"]=json!(match id {Some(id)=>format!("/households/{}/people/{person_id}/health_events/{id}",self.slug),None=>format!("/households/{}/people/{person_id}/health_events",self.slug)});
            Ok::<_,OperationError>(data)
        }.await;
        let data = match result {
            Ok(data) => data,
            Err(error) => return operation_error(error),
        };
        let response = rendering::render(
            self.view,
            self.token,
            "health_events/form.html",
            data,
            error,
        );
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        response
    }

    async fn save(&self, id: Option<&str>, draft: HashMap<String, String>) -> Response {
        if self
            .token
            .verify(forms::field(&draft, "authenticity_token"))
            .is_err()
        {
            return operation_error(OperationError::Forbidden);
        }
        let (principal, tenant) =
            match begin(self.ctx, self.session, self.slug, &self.request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
        let result = async {
            let (person, _) = people::read(&tenant, self.person_id, principal.time_zone()).await?;
            let person_id = person["data"]["id"]
                .as_i64()
                .ok_or(OperationError::Unavailable)?;
            browser::save(&tenant, person_id, id, &draft, principal.provenance()).await?;
            Ok::<_, OperationError>(person_id)
        }
        .await;
        match result {
            Ok(person_id) => {
                if tenant.commit().await.is_err() {
                    return unavailable();
                }
                let language = report_pdf::locale(forms::field(&draft, "locale"));
                (
                    StatusCode::SEE_OTHER,
                    [
                        (
                            header::LOCATION,
                            format!(
                                "/households/{}/people/{person_id}/health_events?locale={language}",
                                self.slug
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
                self.form(id, &Filters::default(), Some(&draft), Some(&error))
                    .await
            }
        }
    }
}

pub(super) async fn new(
    State(ctx): State<AppContext>,
    Path((slug, person_id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(filters): Query<Filters>,
) -> Response {
    FormPage {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        person_id: &person_id,
        request_id: request_id(request),
    }
    .form(None, &filters, None, None)
    .await
}

pub(super) async fn edit(
    State(ctx): State<AppContext>,
    Path((slug, person_id, id)): Path<(String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(filters): Query<Filters>,
) -> Response {
    FormPage {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        person_id: &person_id,
        request_id: request_id(request),
    }
    .form(Some(&id), &filters, None, None)
    .await
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path((slug, person_id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    FormPage {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        person_id: &person_id,
        request_id: request_id(request),
    }
    .save(None, draft)
    .await
}

pub(super) async fn update(
    State(ctx): State<AppContext>,
    Path((slug, person_id, id)): Path<(String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    FormPage {
        ctx: &ctx,
        session: &session,
        token: &token,
        view: &view,
        slug: &slug,
        person_id: &person_id,
        request_id: request_id(request),
    }
    .save(Some(&id), draft)
    .await
}

pub(super) async fn delete(
    State(ctx): State<AppContext>,
    Path((slug, person_id, id)): Path<(String, String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    if token
        .verify(forms::field(&draft, "authenticity_token"))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        let (person, _) = people::read(&tenant, &person_id, principal.time_zone()).await?;
        let person_id = person["data"]["id"]
            .as_i64()
            .ok_or(OperationError::Unavailable)?;
        browser::delete(
            &tenant,
            person_id,
            &id,
            forms::field(&draft, "etag"),
            principal.provenance(),
        )
        .await
    }
    .await;
    match result {
        Ok(()) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            let language = report_pdf::locale(forms::field(&draft, "locale"));
            (
                StatusCode::SEE_OTHER,
                [
                    (
                        header::LOCATION,
                        format!(
                            "/households/{slug}/people/{person_id}/health_events?locale={language}"
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
            operation_error(error)
        }
    }
}
