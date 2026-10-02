use super::*;

fn rejected(
    context: Context,
    location: FormLocation,
    draft: TreatmentDraft,
    errors: Errors,
    status: StatusCode,
) -> Response {
    forms::render(context, location, draft, errors, status)
}

pub(super) fn response_errors(value: &Value) -> Errors {
    let errors = value
        .pointer("/error/errors")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .map(|(name, values)| {
                    (
                        name.clone(),
                        values
                            .as_array()
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_owned)
                                    .collect()
                            })
                            .unwrap_or_default(),
                    )
                })
                .collect::<Errors>()
        })
        .unwrap_or_default();
    if errors.is_empty() {
        BTreeMap::from([(
            "base".into(),
            vec![value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("is invalid")
                .to_owned()],
        )])
    } else {
        errors
    }
}

async fn save(
    state: AppState,
    headers: HeaderMap,
    location: FormLocation,
    fields: HashMap<String, String>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut context = match load(state, headers, &location.slug, &location.person).await {
        Ok(context) => context,
        Err(error) => return error.response(),
    };
    if fields.get("authenticity_token") != Some(&context.api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut draft = TreatmentDraft {
        fields: fields.into_iter().collect(),
    };
    if let Some(id) = &location.id {
        if let Err(error) = source(&mut context, location.kind, id).await {
            return error.response();
        }
        if draft.value("etag").trim().is_empty() {
            return rejected(
                context,
                location,
                draft,
                BTreeMap::from([("base".into(), vec!["missing_browser_precondition".into()])]),
                StatusCode::PRECONDITION_REQUIRED,
            );
        }
    }
    if !draft.value("intent").is_empty() {
        let intent = draft.value("intent").to_owned();
        let (errors, status) = match intentions::apply(&context, &mut draft, &intent) {
            Ok(()) => (Errors::new(), StatusCode::OK),
            Err(errors) => (errors, StatusCode::UNPROCESSABLE_ENTITY),
        };
        return rejected(context, location, draft, errors, status);
    }
    if Uuid::parse_str(draft.value("submission_id")).is_err() {
        return rejected(
            context,
            location,
            draft,
            BTreeMap::from([("base".into(), vec!["missing_browser_precondition".into()])]),
            StatusCode::PRECONDITION_REQUIRED,
        );
    }
    let body = match payload::build(&context, location.kind, &draft) {
        Ok(body) => body,
        Err(errors) => {
            return rejected(
                context,
                location,
                draft,
                errors,
                StatusCode::UNPROCESSABLE_ENTITY,
            )
        }
    };
    let mut extra = HeaderMap::new();
    let key = match HeaderValue::from_str(draft.value("submission_id")) {
        Ok(key) => key,
        Err(_) => {
            return rejected(
                context,
                location,
                draft,
                payload::invalid("base"),
                StatusCode::PRECONDITION_REQUIRED,
            )
        }
    };
    extra.insert("idempotency-key", key);
    if location.id.is_some() {
        let token = match HeaderValue::from_str(draft.value("etag")) {
            Ok(token) => token,
            Err(_) => {
                return rejected(
                    context,
                    location,
                    draft,
                    BTreeMap::from([(
                        "base".into(),
                        vec!["Record has changed since it was last read".into()],
                    )]),
                    StatusCode::CONFLICT,
                )
            }
        };
        extra.insert(header::IF_MATCH, token);
    }
    let path = location.id.as_ref().map_or_else(
        || {
            format!(
                "/api/v1/households/{}/{}",
                context.household_id,
                location.kind.resource()
            )
        },
        |id| {
            format!(
                "/api/v1/households/{}/{}/{}",
                context.household_id,
                location.kind.resource(),
                path_segment(id)
            )
        },
    );
    let csrf = context.api.csrf.clone();
    let reply = match context
        .api
        .call_with_headers(
            if location.id.is_some() {
                Method::PATCH
            } else {
                Method::POST
            },
            &path,
            Some(body),
            Some(&csrf),
            &extra,
        )
        .await
    {
        Ok(reply) => reply,
        Err(error) => return error.response(),
    };
    if reply.status.is_success() {
        if numeric(&reply.value["data"], "person_id") != Some(context.person_id)
            || numeric(&reply.value["data"], "id").is_none()
        {
            return failure(StatusCode::BAD_GATEWAY);
        }
        return redirect(
            format!(
                "/households/{}/people/{}",
                path_segment(&location.slug),
                path_segment(&location.person)
            ),
            context.api.cookie,
        );
    }
    if matches!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY | StatusCode::CONFLICT | StatusCode::PRECONDITION_REQUIRED
    ) {
        if reply.status == StatusCode::UNPROCESSABLE_ENTITY {
            draft
                .fields
                .insert("submission_id".into(), Uuid::new_v4().to_string());
        }
        return rejected(
            context,
            location,
            draft,
            response_errors(&reply.value),
            reply.status,
        );
    }
    if reply.status == StatusCode::UNAUTHORIZED {
        return login_redirect();
    }
    page_status(String::new(), context.api.cookie, reply.status)
}

pub(super) async fn create_assignment(
    State(state): State<AppState>,
    Path((slug, person)): Path<(String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(
        state,
        headers,
        FormLocation {
            slug,
            person,
            id: None,
            kind: Kind::Assignment,
        },
        fields,
    )
    .await
}

pub(super) async fn update_assignment(
    State(state): State<AppState>,
    Path((slug, person, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(
        state,
        headers,
        FormLocation {
            slug,
            person,
            id: Some(id),
            kind: Kind::Assignment,
        },
        fields,
    )
    .await
}

pub(super) async fn create_schedule(
    State(state): State<AppState>,
    Path((slug, person)): Path<(String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(
        state,
        headers,
        FormLocation {
            slug,
            person,
            id: None,
            kind: Kind::Schedule,
        },
        fields,
    )
    .await
}

pub(super) async fn update_schedule(
    State(state): State<AppState>,
    Path((slug, person, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(
        state,
        headers,
        FormLocation {
            slug,
            person,
            id: Some(id),
            kind: Kind::Schedule,
        },
        fields,
    )
    .await
}
