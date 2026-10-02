use super::{Fixture, Html, Selector, Target, fixture, login};
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) struct CurrentBearer {
    owner: Target,
    csrf: String,
    path: String,
    active: bool,
}

impl CurrentBearer {
    pub(super) fn revoke(&mut self) -> bool {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.owner.delete_browser_json(&self.path, &self.csrf)
        }));
        match result {
            Ok(response) if response.status().as_u16() == 204 => {
                self.active = false;
                true
            }
            _ => false,
        }
    }
}

impl Drop for CurrentBearer {
    fn drop(&mut self) {
        if self.active && !self.revoke() {
            eprintln!("failed to revoke current test-only bearer");
        }
    }
}

pub(super) fn fixture_with_current_bearer(client: &str) -> (Fixture, CurrentBearer) {
    let mut fixture = fixture();
    let owner = Target::from_env();
    login(&owner, &fixture, client);
    let api = format!("/api/v1/households/{}", fixture.household_id);
    let me = owner.get(&format!("{api}/me"), None);
    assert_eq!(me.status().as_u16(), 200);
    let me: Value = me.json().unwrap();
    assert_eq!(me["data"]["membership_role"], "owner");
    assert_eq!(me["data"]["account"]["id"], fixture.account_id);
    let inventory = owner.get_html(&format!(
        "/households/{}/medications",
        fixture.household_slug
    ));
    assert_eq!(inventory.status().as_u16(), 200);
    let document = Html::parse_document(&inventory.text().unwrap());
    let csrf = document
        .select(&Selector::parse("meta[name='csrf-token']").unwrap())
        .next()
        .and_then(|token| token.value().attr("content"))
        .expect("fresh owner API CSRF")
        .to_owned();
    let response = owner.post_browser_json(
        &format!("{api}/admin/app_tokens"),
        &csrf,
        &json!({"api_app_token": {"name": format!("Completion current bearer {client}")}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created = response.json::<Value>().unwrap()["data"].clone();
    let guard = CurrentBearer {
        owner,
        csrf,
        path: format!("{api}/admin/app_tokens/{}", created["id"].as_i64().unwrap()),
        active: true,
    };
    fixture.access_token = created["token"]
        .as_str()
        .expect("one-time current bearer")
        .to_owned();
    let readback = Target::from_env().get(&format!("{api}/me"), Some(&fixture.access_token));
    assert_eq!(readback.status().as_u16(), 200);
    assert_eq!(
        readback.json::<Value>().unwrap()["data"]["account"]["id"],
        fixture.account_id
    );
    (fixture, guard)
}
