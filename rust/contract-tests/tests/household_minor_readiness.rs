use medtracker_contract_tests::{Fixture, Target, fixture};
use scraper::{Html, Selector};
use serde_json::Value;
use std::env;

struct TemporaryMinor {
    db: postgres::Client,
    person_id: i64,
    original_type: i32,
    original_birth_date: Option<String>,
    original_capacity: bool,
    restored: bool,
}

impl TemporaryMinor {
    fn for_viewer(fixture: &Fixture) -> Self {
        let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL");
        let mut db = postgres::Client::connect(&url, postgres::NoTls).expect("contract database");
        let previous = db
            .query_one(
                "SELECT id, person_type, date_of_birth::text, has_capacity FROM people WHERE id = (SELECT person_id FROM household_memberships WHERE id = $1)",
                &[&fixture.view_membership_id],
            )
            .expect("viewer person");
        let person_id = previous.get("id");
        let original_type = previous.get("person_type");
        let original_birth_date = previous.get("date_of_birth");
        let original_capacity = previous.get("has_capacity");
        db.execute(
            "UPDATE people SET person_type = 1, date_of_birth = (CURRENT_DATE - INTERVAL '10 years')::date, has_capacity = false WHERE id = $1",
            &[&person_id],
        )
        .expect("make viewer under 18");
        Self {
            db,
            person_id,
            original_type,
            original_birth_date,
            original_capacity,
            restored: false,
        }
    }

    fn restore(&mut self) {
        let updated = self
            .db
            .execute(
                "UPDATE people SET person_type = $2, date_of_birth = $3::text::date, has_capacity = $4 WHERE id = $1",
                &[
                    &self.person_id,
                    &self.original_type,
                    &self.original_birth_date,
                    &self.original_capacity,
                ],
            )
            .expect("restore viewer person");
        assert_eq!(updated, 1);
        self.restored = true;
    }
}

impl Drop for TemporaryMinor {
    fn drop(&mut self) {
        if self.restored {
            return;
        }
        if let Err(error) = self.db.execute(
            "UPDATE people SET person_type = $2, date_of_birth = $3::text::date, has_capacity = $4 WHERE id = $1",
            &[
                &self.person_id,
                &self.original_type,
                &self.original_birth_date,
                &self.original_capacity,
            ],
        ) {
            eprintln!("failed to restore temporary minor: {error}");
        }
    }
}


fn input(html: &str, name: &str) -> String {
    Html::parse_document(html).select(&Selector::parse(&format!("input[name='{name}']")).unwrap())
        .next().and_then(|node| node.value().attr("value")).expect("native login field").to_owned()
}

#[test]
fn minor_can_read_authorised_person_and_assignments_when_schedule_index_is_forbidden() {
    let fixture = fixture();
    let mut minor = TemporaryMinor::for_viewer(&fixture);
    let email: String = minor.db.query_one("SELECT email FROM accounts WHERE id = $1", &[&fixture.view_account_id])
        .expect("actual viewer email").get(0);
    let target = Target::from_env();
    let html = target.get_html("/login").text().unwrap();
    let response = target.post_html_form_from_client("/login", "198.18.52.1", &[
        ("authenticity_token".into(), input(&html, "authenticity_token")),
        ("email".into(), email), ("password".into(), "password".into()),
    ]);
    assert_eq!(response.status().as_u16(), 302);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let person = target.get(&format!("{api}/people/{}", fixture.managed_person_id), None);
    assert_eq!(person.status().as_u16(), 200);
    let person = person.json::<Value>().unwrap()["data"].clone();
    assert_eq!(person["id"], fixture.managed_person_id);
    let assignments = target.get(&format!("{api}/person_medications?per_page=100"), None);
    assert_eq!(assignments.status().as_u16(), 200);
    let assignments = assignments.json::<Value>().unwrap()["data"].clone();
    let visible = assignments.as_array().unwrap().iter().filter(|row| row["person_id"] == fixture.managed_person_id).collect::<Vec<_>>();
    assert!(!visible.is_empty(), "fixture exposes an actual readable assignment");
    let schedules = target.get(&format!("{api}/schedules?per_page=100"), None);
    assert_eq!(schedules.status().as_u16(), 403);
    let denied = schedules.json::<Value>().unwrap();
    assert!(denied.get("data").is_none(), "schedule denial must not disclose source rows");
    for (locale, guidance) in [
        ("en", "Schedules are not available with your current access. Assignments shown here remain readable."),
        ("cy", "Nid yw amserlenni ar gael gyda'ch mynediad presennol. Mae'r aseiniadau a ddangosir yma yn parhau i fod yn ddarllenadwy."),
        ("ga", "Níl sceidil ar fáil leis an rochtain atá agat faoi láthair. Is féidir na sannacháin a thaispeántar anseo a léamh fós."),
        ("es", "Los horarios no están disponibles con su acceso actual. Las asignaciones que se muestran aquí siguen siendo legibles."),
        ("pt", "Os horários não estão disponíveis com o seu acesso atual. As atribuições apresentadas aqui continuam a poder ser lidas."),
    ] {
        let response = target.get_html_with_header(&format!("/households/{}/people/{}", fixture.household_slug, fixture.managed_person_id), "Accept-Language", locale);
        let status = response.status().as_u16();
        let html = response.text().unwrap();
        assert_eq!(status, 200, "Readable person page must survive schedule-only denial in {locale}");
        let document = Html::parse_document(&html);
        let text = document.root_element().text().collect::<Vec<_>>().join(" ");
        assert!(text.contains(person["name"].as_str().unwrap()));
        assert!(text.contains(guidance), "Schedule omission needs truthful guidance in {locale}");
        for source in &visible {
            let id = source["id"].as_i64().unwrap();
            let selector = Selector::parse(&format!("a[href='/households/{}/people/{}/assignments/{id}/history']", fixture.household_slug, fixture.managed_person_id)).unwrap();
            assert_eq!(document.select(&selector).count(), 1, "Readable assignment history stays available");
        }
        assert_eq!(document.select(&Selector::parse("a[href*='/schedules/']").unwrap()).count(), 0);
    }
    for id in [fixture.hidden_person_id, fixture.foreign_person_id] {
        assert_eq!(target.get_html(&format!("/households/{}/people/{id}", fixture.household_slug)).status().as_u16(), 404, "person authorisation remains mandatory");
    }
    assert_eq!(target.get(&format!("{api}/schedules?per_page=100"), None).status().as_u16(), 403);
    minor.restore();
}
