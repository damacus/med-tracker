use medtracker_contract_tests::{Fixture, Target, fixture};
use scraper::{Html, Selector};
use serde_json::{Value, json};

fn field(html: &str, name: &str) -> String {
    let document = Html::parse_document(html);
    let node = document
        .select(&Selector::parse(&format!("[name='{name}']")).unwrap())
        .next()
        .expect("native field");
    if node.value().name() == "textarea" {
        node.text().collect()
    } else if node.value().name() == "select" {
        node.select(&Selector::parse("option[selected]").unwrap())
            .next()
            .and_then(|option| option.value().attr("value"))
            .expect("selected value")
            .to_owned()
    } else {
        node.value().attr("value").expect("field value").to_owned()
    }
}

fn read(target: &Target, path: &str) -> Value {
    let response = target.get(path, None);
    assert_eq!(response.status().as_u16(), 200);
    response.json::<Value>().unwrap()["data"].clone()
}

fn setup(name: &str, client: &str) -> (Fixture, Target, String, Value) {
    let fixture = fixture();
    let target = Target::from_env();
    let html = target.get_html("/login").text().unwrap();
    assert_eq!(
        target
            .post_html_form_from_client(
                "/login",
                client,
                &[
                    (
                        "authenticity_token".into(),
                        field(&html, "authenticity_token")
                    ),
                    ("email".into(), fixture.primary_email.clone()),
                    ("password".into(), "password".into()),
                ]
            )
            .status()
            .as_u16(),
        302
    );
    let html = target
        .get_html(&format!(
            "/households/{}/medications",
            fixture.household_slug
        ))
        .text()
        .unwrap();
    let document = Html::parse_document(&html);
    let csrf = document
        .select(&Selector::parse("meta[name='csrf-token']").unwrap())
        .next()
        .unwrap()
        .value()
        .attr("content")
        .unwrap()
        .to_owned();
    let response = target.post_browser_json(&format!("/api/v1/households/{}/medications", fixture.household_id), &csrf,
        &json!({"medication": {"name": name, "location_id": fixture.primary_location_id, "dose_amount": "1.25", "dose_unit": "ml", "current_supply": "200", "reorder_threshold": "3"}}));
    assert_eq!(response.status().as_u16(), 201);
    let medication = response.json::<Value>().unwrap()["data"].clone();
    (fixture, target, csrf, medication)
}

fn prefix(fixture: &Fixture) -> String {
    format!(
        "/households/{}/people/{}",
        fixture.household_slug, fixture.journey_browser_person_id
    )
}

fn form(target: &Target, path: &str) -> String {
    let response = target.get_html(path);
    assert_eq!(
        response.status().as_u16(),
        200,
        "ordinary treatment editor exists"
    );
    let html = response.text().unwrap();
    assert_eq!(
        Html::parse_document(&html)
            .select(&Selector::parse("form.household-form").unwrap())
            .count(),
        1
    );
    html
}

fn common(csrf: &str, medication: &Value) -> Vec<(String, String)> {
    vec![
        ("authenticity_token".into(), csrf.into()),
        (
            "medication_id".into(),
            medication["id"].as_i64().unwrap().to_string(),
        ),
        ("dose_amount".into(), "1.25".into()),
        ("dose_unit".into(), "ml".into()),
        ("max_daily_doses".into(), "4".into()),
        ("min_hours_between_doses".into(), "8".into()),
        ("dose_cycle".into(), "daily".into()),
        ("notes".into(), "Treatment <script>draft</script>".into()),
    ]
}

fn find_created(target: &Target, fixture: &Fixture, resource: &str, medication: &Value) -> Value {
    let mut records = Vec::new();
    for page in 1..=5 {
        let response = target.get(
            &format!(
                "/api/v1/households/{}/{resource}?page={page}&per_page=100",
                fixture.household_id
            ),
            None,
        );
        assert_eq!(response.status().as_u16(), 200);
        let body = response.json::<Value>().unwrap();
        let total = body["meta"]["total_count"]
            .as_u64()
            .expect("collection total") as usize;
        let batch = body["data"].as_array().unwrap();
        assert!(!batch.is_empty() || records.len() == total);
        records.extend(batch.iter().cloned());
        if records.len() >= total {
            let matching = records
                .into_iter()
                .filter(|record| {
                    record["medication_id"] == medication["id"]
                        && record["person_id"] == fixture.journey_browser_person_id
                })
                .collect::<Vec<_>>();
            assert_eq!(matching.len(), 1);
            return matching[0].clone();
        }
    }
    panic!("collection did not reach its authoritative total");
}

#[test]
fn direct_assignment_creates_edits_and_keeps_invalid_and_stale_drafts() {
    let (fixture, target, csrf, medication) = setup("Direct treatment round trip", "198.18.41.1");
    let base = format!("{}/assignments", prefix(&fixture));
    let html = form(&target, &format!("{base}/new"));
    let mut fields = common(&csrf, &medication);
    fields.push(("administration_kind".into(), "as_needed".into()));
    fields.push(("submission_id".into(), field(&html, "submission_id")));
    let response = target.post_browser_form(&base, &fields);
    assert_eq!(response.status().as_u16(), 303);
    let saved = find_created(&target, &fixture, "person_medications", &medication);
    assert_eq!(saved["dose_amount"], "1.25");
    assert_eq!(saved["dose_unit"], "ml");
    assert_eq!(saved["administration_kind"], "as_needed");
    assert_eq!(saved["max_daily_doses"], 4);
    assert_eq!(saved["min_hours_between_doses"], "8.0");
    assert_eq!(saved["dose_cycle"], "daily");
    assert_eq!(saved["notes"], "Treatment <script>draft</script>");
    let member = format!("{base}/{}", saved["id"].as_i64().unwrap());
    let html = form(&target, &format!("{member}/edit"));
    fields.push(("etag".into(), field(&html, "etag")));
    fields.retain(|(name, _)| name != "submission_id");
    fields.push(("submission_id".into(), field(&html, "submission_id")));
    fields
        .iter_mut()
        .find(|(name, _)| name == "dose_amount")
        .unwrap()
        .1 = "1.234".into();
    let response = target.post_browser_form(&member, &fields);
    assert_eq!(response.status().as_u16(), 422);
    let rejected = response.text().unwrap();
    assert_eq!(field(&rejected, "dose_amount"), "1.234");
    assert_eq!(
        field(&rejected, "notes"),
        "Treatment <script>draft</script>"
    );
    assert!(!rejected.contains("<script>draft</script>"));
    let api = format!(
        "/api/v1/households/{}/person_medications/{}",
        fixture.household_id,
        saved["id"].as_i64().unwrap()
    );
    assert_eq!(read(&target, &api), saved);
    fields
        .iter_mut()
        .find(|(name, _)| name == "submission_id")
        .unwrap()
        .1 = field(&form(&target, &format!("{member}/edit")), "submission_id");
    let response = target.patch_json(
        &api,
        &fixture.access_token,
        &json!({"person_medication": {"notes": "Other update"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let changed = response.json::<Value>().unwrap()["data"].clone();
    fields
        .iter_mut()
        .find(|(name, _)| name == "dose_amount")
        .unwrap()
        .1 = "2.50".into();
    let response = target.post_browser_form(&member, &fields);
    assert_eq!(response.status().as_u16(), 409);
    let rejected = response.text().unwrap();
    assert_eq!(field(&rejected, "dose_amount"), "2.50");
    assert_eq!(field(&rejected, "etag"), field(&html, "etag"));
    assert_eq!(read(&target, &api), changed);
    let html = form(&target, &format!("{member}/edit"));
    fields
        .iter_mut()
        .find(|(name, _)| name == "etag")
        .unwrap()
        .1 = field(&html, "etag");
    fields
        .iter_mut()
        .find(|(name, _)| name == "submission_id")
        .unwrap()
        .1 = field(&html, "submission_id");
    assert_eq!(
        target.post_browser_form(&member, &fields).status().as_u16(),
        303
    );
    let updated = read(&target, &api);
    assert_eq!(updated["dose_amount"], "2.5");
    assert_eq!(updated["dose_cycle"], "daily");
}

fn schedule_round_trip(kind: &str, client: &str) {
    let (fixture, target, csrf, medication) =
        setup(&format!("{kind} treatment round trip"), client);
    let base = format!("{}/schedules", prefix(&fixture));
    let html = form(&target, &format!("{base}/new?type={kind}"));
    let mut fields = common(&csrf, &medication);
    fields.extend([
        ("submission_id".into(), field(&html, "submission_id")),
        ("schedule_type".into(), kind.into()),
        ("frequency".into(), "Entered frequency".into()),
        ("start_date".into(), "2030-03-30".into()),
        ("end_date".into(), "2030-04-10".into()),
        ("time_0".into(), "08:00".into()),
    ]);
    let config = match kind {
        "daily" | "every_other_day" => json!({"times": ["08:00"]}),
        "multiple_daily" => {
            fields.push(("time_1".into(), "20:00".into()));
            json!({"times": ["08:00", "20:00"]})
        }
        "weekly" => {
            fields.extend([
                ("weekday_monday".into(), "true".into()),
                ("weekday_friday".into(), "true".into()),
            ]);
            json!({"times": ["08:00"], "weekdays": ["monday", "friday"]})
        }
        "specific_dates" => {
            fields.extend([
                ("date_0".into(), "2030-03-31".into()),
                ("date_1".into(), "2030-04-02".into()),
            ]);
            json!({"times": ["08:00"], "dates": ["2030-03-31", "2030-04-02"]})
        }
        "prn" => {
            fields.retain(|(name, _)| name != "time_0");
            json!({"as_needed": true})
        }
        "tapering" => {
            for (index, start, end, amount, time) in [
                (0, "2030-03-30", "2030-03-31", "1.25", "08:00"),
                (1, "2030-04-01", "2030-04-10", "0.75", "09:00"),
            ] {
                for (name, value) in [
                    ("start_date", start),
                    ("end_date", end),
                    ("dose_amount", amount),
                    ("dose_unit", "ml"),
                    ("max_daily_doses", "2"),
                    ("min_hours_between_doses", "8"),
                    ("time_0", time),
                ] {
                    fields.push((format!("step_{index}_{name}"), value.into()));
                }
            }
            fields.retain(|(name, _)| name != "time_0");
            json!({"taper_steps": [
                {"start_date": "2030-03-30", "end_date": "2030-03-31", "dose_amount": "1.25", "dose_unit": "ml", "max_daily_doses": 2, "min_hours_between_doses": "8", "times": ["08:00"]},
                {"start_date": "2030-04-01", "end_date": "2030-04-10", "dose_amount": "0.75", "dose_unit": "ml", "max_daily_doses": 2, "min_hours_between_doses": "8", "times": ["09:00"]}
            ]})
        }
        _ => unreachable!(),
    };
    assert_eq!(
        target.post_browser_form(&base, &fields).status().as_u16(),
        303
    );
    let saved = find_created(&target, &fixture, "schedules", &medication);
    assert_eq!(saved["schedule_type"], kind);
    assert_eq!(saved["schedule_config"], config);
    assert_eq!(saved["start_date"], "2030-03-30");
    assert_eq!(saved["end_date"], "2030-04-10");
    assert_eq!(saved["dose_amount"], "1.25");
    assert_eq!(saved["min_hours_between_doses"], "8.0");
    let member = format!("{base}/{}", saved["id"].as_i64().unwrap());
    let html = form(&target, &format!("{member}/edit"));
    fields.retain(|(name, _)| name != "submission_id");
    fields.extend([
        ("etag".into(), field(&html, "etag")),
        ("submission_id".into(), field(&html, "submission_id")),
    ]);
    fields
        .iter_mut()
        .find(|(name, _)| name == "start_date")
        .unwrap()
        .1 = "2030-19-44".into();
    let response = target.post_browser_form(&member, &fields);
    assert_eq!(response.status().as_u16(), 422);
    let rejected = response.text().unwrap();
    assert_eq!(field(&rejected, "start_date"), "2030-19-44");
    if kind == "tapering" {
        assert_eq!(field(&rejected, "step_0_dose_amount"), "1.25");
        assert_eq!(field(&rejected, "step_1_dose_amount"), "0.75");
        assert_eq!(field(&rejected, "step_1_time_0"), "09:00");
    }
    let api = format!(
        "/api/v1/households/{}/schedules/{}",
        fixture.household_id,
        saved["id"].as_i64().unwrap()
    );
    assert_eq!(read(&target, &api), saved);
    fields
        .iter_mut()
        .find(|(name, _)| name == "start_date")
        .unwrap()
        .1 = "2030-03-30".into();
    fields
        .iter_mut()
        .find(|(name, _)| name == "notes")
        .unwrap()
        .1 = "Edited schedule".into();
    fields
        .iter_mut()
        .find(|(name, _)| name == "submission_id")
        .unwrap()
        .1 = field(&form(&target, &format!("{member}/edit")), "submission_id");
    assert_eq!(
        target.post_browser_form(&member, &fields).status().as_u16(),
        303
    );
    let updated = read(&target, &api);
    assert_eq!(updated["notes"], "Edited schedule");
    assert_eq!(updated["schedule_config"], config);
    assert_eq!(updated["end_date"], "2030-04-10");
}

macro_rules! schedule_test {
    ($name:ident, $kind:literal, $client:literal) => {
        #[test]
        fn $name() {
            schedule_round_trip($kind, $client);
        }
    };
}

fn ordinary_blank_fields(csrf: &str, medication: &Value, html: &str) -> Vec<(String, String)> {
    let mut fields = common(csrf, medication);
    for (name, value) in &mut fields {
        if matches!(
            name.as_str(),
            "max_daily_doses" | "min_hours_between_doses" | "notes"
        ) {
            value.clear();
        }
        if name == "dose_cycle" {
            *value = field(html, "dose_cycle");
        }
    }
    fields.push(("submission_id".into(), field(html, "submission_id")));
    fields
}

fn blank_optionals(schedule: bool, client: &str) {
    let (fixture, target, csrf, medication) =
        setup("Ordinary treatment with blank guidance", client);
    let resource = if schedule { "schedules" } else { "assignments" };
    let base = format!("{}/{resource}", prefix(&fixture));
    let html = form(&target, &format!("{base}/new"));
    let mut fields = ordinary_blank_fields(&csrf, &medication, &html);
    if schedule {
        fields.extend([
            ("schedule_type".into(), "daily".into()),
            ("start_date".into(), "2030-03-30".into()),
            ("end_date".into(), "2030-04-30".into()),
            ("frequency".into(), String::new()),
            ("time_0".into(), "08:00".into()),
        ]);
    } else {
        fields.push(("administration_kind".into(), "as_needed".into()));
    }
    let response = target.post_browser_form(&base, &fields);
    assert_eq!(
        response.status().as_u16(),
        303,
        "ordinary form accepts optional fields left blank"
    );
    let saved = find_created(
        &target,
        &fixture,
        if schedule {
            "schedules"
        } else {
            "person_medications"
        },
        &medication,
    );
    if schedule {
        assert_eq!(saved["max_daily_doses"], 4);
    } else {
        assert!(saved["max_daily_doses"].is_null());
    }
    assert!(saved["min_hours_between_doses"].is_null());
    assert_eq!(saved["notes"], "");
    assert_eq!(saved["dose_cycle"], "daily");
    if schedule {
        assert_eq!(saved["end_date"], "2030-04-30");
        assert_eq!(saved["frequency"], "");
        assert_eq!(saved["schedule_config"], json!({"times": ["08:00"]}));
    }
}

fn null_cycle_edit(schedule: bool, client: &str) {
    let (fixture, target, csrf, medication) = setup("Existing treatment without a cycle", client);
    let resource = if schedule {
        "schedules"
    } else {
        "person_medications"
    };
    let body_key = if schedule {
        "schedule"
    } else {
        "person_medication"
    };
    let mut attributes = json!({"person_id": fixture.journey_browser_person_id.to_string(), "medication_id": medication["id"].as_i64().unwrap().to_string(), "dose_amount": "1.25", "dose_unit": "ml", "notes": "Existing notes"});
    if schedule {
        attributes["schedule_type"] = "daily".into();
        attributes["schedule_config"] = json!({"times": ["08:00"]});
        attributes["start_date"] = "2030-03-30".into();
        attributes["end_date"] = "2030-04-30".into();
    } else {
        attributes["administration_kind"] = "as_needed".into();
    }
    let response = target.post_browser_json(
        &format!("/api/v1/households/{}/{resource}", fixture.household_id),
        &csrf,
        &json!({(body_key): attributes}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let original = response.json::<Value>().unwrap()["data"].clone();
    assert!(original["dose_cycle"].is_null());
    let base = format!(
        "{}/{}/{}",
        prefix(&fixture),
        if schedule { "schedules" } else { "assignments" },
        original["id"].as_i64().unwrap()
    );
    let html = form(&target, &format!("{base}/edit"));
    assert_eq!(field(&html, "dose_cycle"), "");
    let mut fields = ordinary_blank_fields(&csrf, &medication, &html);
    fields.push(("etag".into(), field(&html, "etag")));
    if schedule {
        fields.extend([
            ("schedule_type".into(), "daily".into()),
            ("start_date".into(), "2030-03-30".into()),
            ("end_date".into(), "2030-04-30".into()),
            ("frequency".into(), String::new()),
            ("time_0".into(), "08:00".into()),
            ("config_original".into(), field(&html, "config_original")),
            (
                "config_type_original".into(),
                field(&html, "config_type_original"),
            ),
        ]);
    } else {
        fields.push(("administration_kind".into(), "as_needed".into()));
    }
    assert_eq!(
        target.post_browser_form(&base, &fields).status().as_u16(),
        303,
        "editing unrelated fields preserves an absent dose cycle"
    );
    let saved = read(
        &target,
        &format!(
            "/api/v1/households/{}/{resource}/{}",
            fixture.household_id,
            original["id"].as_i64().unwrap()
        ),
    );
    assert!(saved["dose_cycle"].is_null());
    assert_eq!(saved["notes"], "");
    assert_eq!(saved["dose_amount"], original["dose_amount"]);
    if schedule {
        assert_eq!(saved["schedule_config"], original["schedule_config"]);
        assert_eq!(saved["start_date"], original["start_date"]);
        assert_eq!(saved["end_date"], original["end_date"]);
    }
}

#[test]
fn ordinary_assignment_can_leave_optional_guidance_blank() {
    blank_optionals(false, "198.18.41.10");
}
#[test]
fn ordinary_schedule_can_leave_optional_guidance_blank() {
    blank_optionals(true, "198.18.41.11");
}
#[test]
fn assignment_edit_preserves_existing_null_cycle() {
    null_cycle_edit(false, "198.18.41.12");
}
#[test]
fn schedule_edit_preserves_existing_null_cycle() {
    null_cycle_edit(true, "198.18.41.13");
}

#[test]
fn blank_required_end_date_keeps_original_draft_token_key_and_saved_schedule() {
    let (fixture, target, csrf, medication) =
        setup("Blank end date rejects schedule edit", "198.18.41.14");
    let api_base = format!("/api/v1/households/{}/schedules", fixture.household_id);
    let response = target.post_browser_json(&api_base, &csrf, &json!({"schedule": {
        "person_id": fixture.journey_browser_person_id.to_string(), "medication_id": medication["id"].as_i64().unwrap().to_string(), "dose_amount": "1.25", "dose_unit": "ml", "start_date": "2030-03-30", "end_date": "2030-04-30", "schedule_type": "daily", "schedule_config": {"times": ["08:00"]}, "notes": "Saved original schedule", "dose_cycle": "daily", "max_daily_doses": 4, "min_hours_between_doses": "8"
    }}));
    assert_eq!(response.status().as_u16(), 201);
    let original = response.json::<Value>().unwrap()["data"].clone();
    let api = format!("{api_base}/{}", original["id"].as_i64().unwrap());
    let member = format!(
        "{}/schedules/{}",
        prefix(&fixture),
        original["id"].as_i64().unwrap()
    );
    let html = form(&target, &format!("{member}/edit"));
    let mut fields = common(&csrf, &medication);
    fields.extend([
        ("etag".into(), field(&html, "etag")),
        ("submission_id".into(), field(&html, "submission_id")),
        ("schedule_type".into(), "daily".into()),
        ("start_date".into(), "2030-03-30".into()),
        ("end_date".into(), String::new()),
        ("frequency".into(), "Entered frequency".into()),
        ("time_0".into(), "08:00".into()),
        ("config_original".into(), field(&html, "config_original")),
        (
            "config_type_original".into(),
            field(&html, "config_type_original"),
        ),
    ]);
    let response = target.post_browser_form(&member, &fields);
    assert_eq!(response.status().as_u16(), 422);
    let rejected = response.text().unwrap();
    for (name, value) in &fields {
        assert_eq!(
            field(&rejected, name),
            *value,
            "blank end rejected draft {name}"
        );
    }
    assert_eq!(read(&target, &api), original);
}

schedule_test!(daily_schedule_round_trip, "daily", "198.18.41.2");
schedule_test!(
    multiple_daily_schedule_round_trip,
    "multiple_daily",
    "198.18.41.3"
);
schedule_test!(weekly_schedule_round_trip, "weekly", "198.18.41.4");
schedule_test!(
    specific_date_schedule_round_trip,
    "specific_dates",
    "198.18.41.5"
);
schedule_test!(as_needed_schedule_round_trip, "prn", "198.18.41.6");
schedule_test!(taper_schedule_round_trip, "tapering", "198.18.41.7");
schedule_test!(
    every_other_day_schedule_round_trip,
    "every_other_day",
    "198.18.41.8"
);

#[test]
fn assignment_pause_and_resume_preserve_reason_history_and_replay() {
    let (fixture, target, csrf, medication) = setup("Treatment pause history", "198.18.41.9");
    let api_base = format!("/api/v1/households/{}", fixture.household_id);
    let response = target.post_browser_json(&format!("{api_base}/person_medications"), &csrf,
        &json!({"person_medication": {"person_id": fixture.journey_browser_person_id.to_string(), "medication_id": medication["id"].as_i64().unwrap().to_string(), "dose_amount": "1.25", "dose_unit": "ml", "administration_kind": "as_needed"}}));
    assert_eq!(response.status().as_u16(), 201);
    let source = response.json::<Value>().unwrap()["data"].clone();
    let member = format!(
        "{}/assignments/{}",
        prefix(&fixture),
        source["id"].as_i64().unwrap()
    );
    let html = form(&target, &format!("{member}/pause"));
    let fields = vec![
        ("authenticity_token".into(), csrf.clone()),
        ("etag".into(), field(&html, "etag")),
        ("source_type".into(), field(&html, "source_type")),
        ("source_id".into(), field(&html, "source_id")),
        ("submission_id".into(), field(&html, "submission_id")),
        ("reason".into(), "clinician_advice".into()),
        ("note".into(), "Review next week".into()),
    ];
    assert_eq!(
        target
            .post_browser_form(&format!("{member}/pause"), &fields)
            .status()
            .as_u16(),
        303
    );
    assert_eq!(
        target
            .post_browser_form(&format!("{member}/pause"), &fields)
            .status()
            .as_u16(),
        303
    );
    let periods = read(
        &target,
        &format!(
            "{api_base}/medication_pause_periods?source_type=person_medication&source_id={}",
            source["portable_id"].as_str().unwrap()
        ),
    );
    assert_eq!(periods.as_array().unwrap().len(), 1);
    assert_eq!(periods[0]["reason"], "clinician_advice");
    assert_eq!(periods[0]["note"], "Review next week");
    assert!(periods[0]["ended_at"].is_null());
    let html = form(&target, &format!("{member}/resume"));
    let resume = vec![
        ("authenticity_token".into(), csrf),
        ("etag".into(), field(&html, "etag")),
        ("source_type".into(), field(&html, "source_type")),
        ("source_id".into(), field(&html, "source_id")),
        ("period_etag".into(), field(&html, "period_etag")),
        ("pause_period_id".into(), field(&html, "pause_period_id")),
        ("submission_id".into(), field(&html, "submission_id")),
    ];
    assert_eq!(
        target
            .post_browser_form(&format!("{member}/resume"), &resume)
            .status()
            .as_u16(),
        303
    );
    assert_eq!(
        target
            .post_browser_form(&format!("{member}/resume"), &resume)
            .status()
            .as_u16(),
        303
    );
    let closed = read(
        &target,
        &format!(
            "{api_base}/medication_pause_periods?source_type=person_medication&source_id={}",
            source["portable_id"].as_str().unwrap()
        ),
    );
    assert_eq!(closed.as_array().unwrap().len(), 1);
    assert!(closed[0]["ended_at"].is_string());
    let history = target.get_html(&format!("{member}/history"));
    assert_eq!(history.status().as_u16(), 200);
    assert!(history.text().unwrap().contains("Review next week"));
}
