use loco_rs::{controller::views::ViewRenderer, prelude::TeraView};
use serde_json::json;

#[test]
fn person_carer_pages_render_in_every_supported_locale() {
    let view = TeraView::build().unwrap();
    for (locale, source) in [
        ("en", include_str!("../../assets/reports/locales/en.yml")),
        ("cy", include_str!("../../assets/reports/locales/cy.yml")),
        ("ga", include_str!("../../assets/reports/locales/ga.yml")),
        ("pt", include_str!("../../assets/reports/locales/pt.yml")),
        ("es", include_str!("../../assets/reports/locales/es.yml")),
    ] {
        let all: serde_json::Value = serde_yaml_ng::from_str(source).unwrap();
        let labels = &all[locale];
        for manager in [true, false] {
            for populated in [true, false] {
                let data = json!({
                    "slug":"synthetic-household","person":{"id":73002,"name":"Synthetic minor"},
                    "lang":locale,"title":labels["people"]["carer_relationships"]["new_title"],
                    "labels":labels["people"]["carer_relationships"],
                    "back_label":labels["people"]["show"]["back"],
                    "care_warning_labels":labels["care_warning"],
                    "care_warning":{"person_id":73002,"person_name":"Synthetic minor","can_assign":true},
                    "manager":manager,"draft":{"carer_id":"","email":"","relationship_type":"parent"},
                    "error":null,"allow_palette":true,"appearances":[],"palettes":[],
                    "authenticity_token":"synthetic-render-token",
                    "options":[{"id":73004,"name":"Synthetic parent","selected":false}],
                    "records":if populated {json!([{"id":86001,"name":"Synthetic parent","kind":"parent","active":true,"version":"synthetic-version"}])} else {json!([])}
                });
                let rendered = view
                    .render("person_carers/index.html", data)
                    .unwrap_or_else(|error| {
                        panic!("{locale}, manager={manager}, populated={populated}: {error:?}")
                    });
                assert!(
                    rendered.contains(
                        labels["people"]["carer_relationships"]["new_title"]
                            .as_str()
                            .unwrap()
                    )
                );
            }
        }
    }
}

async fn parent_assignment_fixture(app: &super::Application) {
    use sea_orm::ConnectionTrait;
    app.fixture.admin.execute_unprepared(
        "UPDATE household_memberships SET role='member' WHERE id=74001;
         INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78005,72001,74001,73002,'manage','parent',now(),now());
         INSERT INTO accounts(id,email,status,created_at,updated_at) VALUES(71003,'parent-candidate@example.test',2,now(),now());
         INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,71003,72001,'Synthetic candidate',0,true,now(),now());
         INSERT INTO users(id,person_id,email_address,password_digest,created_at,updated_at) VALUES(77002,73004,'parent-candidate@example.test',crypt('password',gen_salt('bf',4)),now(),now());
         INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74002,71003,72001,73004,'member','active',now(),now(),now());"
    ).await.unwrap();
}

#[tokio::test]
async fn person_carer_nonmanager_cannot_restore_or_assign_ineligible_existing_accounts() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            person_carers,
        },
    };
    use sea_orm::ConnectionTrait;
    let app = super::Application::new().await;
    parent_assignment_fixture(&app).await;
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-parent-restrictions".into(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".into(),
    };
    let target = url::Url::parse("http://127.0.0.1/invitations/accept").unwrap();
    let draft = std::collections::HashMap::from([
        ("email".into(), "parent-candidate@example.test".into()),
        ("relationship_type".into(), "professional_carer".into()),
    ]);
    for mutation in [
        "UPDATE people SET professional_title='Clinician' WHERE id=73004",
        "UPDATE people SET professional_title=NULL,has_capacity=false WHERE id=73004",
        "UPDATE people SET has_capacity=true,person_type=1 WHERE id=73004",
        "UPDATE people SET person_type=0 WHERE id=73004; UPDATE accounts SET status=1 WHERE id=71003",
        "UPDATE accounts SET status=2 WHERE id=71003; UPDATE users SET active=false WHERE id=77002",
        "UPDATE users SET active=true WHERE id=77002; UPDATE household_memberships SET revoked_at=now() WHERE id=74002",
        "UPDATE household_memberships SET revoked_at=NULL,status='suspended' WHERE id=74002",
        "UPDATE household_memberships SET status='active' WHERE id=74002; INSERT INTO carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at) VALUES(86002,72001,73004,73004,'self',true,now(),now())",
        "DELETE FROM carer_relationships WHERE id=86002; INSERT INTO carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at) VALUES(86001,72001,73004,73002,'parent',false,now(),now())",
    ] {
        app.fixture
            .admin
            .execute_unprepared(mutation)
            .await
            .unwrap();
        let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
        let result = person_carers::assign(&tenant, 73002, &draft, &target, &provenance).await;
        tenant.rollback().await.unwrap();
        assert!(result.is_err(), "{mutation}");
    }
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    assert!(
        person_carers::change(&tenant, 73002, 86001, true, "", &provenance)
            .await
            .is_err()
    );
    tenant.rollback().await.unwrap();
    let row = app.fixture.admin.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM carer_relationships WHERE active) AS active,(SELECT count(*) FROM versions WHERE item_type='CarerRelationship') AS audits"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "active").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 0);
    app.close().await;
}

#[tokio::test]
async fn person_carer_nonmanager_ignores_forged_role_and_rejects_unmanaged_or_adult_people() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            person_carers,
        },
    };
    use sea_orm::ConnectionTrait;
    let app = super::Application::new().await;
    parent_assignment_fixture(&app).await;
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-parent-scope".into(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".into(),
    };
    let target = url::Url::parse("http://127.0.0.1/invitations/accept").unwrap();
    let draft = std::collections::HashMap::from([
        ("email".into(), "parent-candidate@example.test".into()),
        ("relationship_type".into(), "professional_carer".into()),
        ("carer_id".into(), "73001".into()),
    ]);
    for person_id in [73001, 73003, 999999] {
        let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
        assert!(
            person_carers::assign(&tenant, person_id, &draft, &target, &provenance)
                .await
                .is_err()
        );
        tenant.rollback().await.unwrap();
    }
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    assert!(matches!(
        person_carers::assign(&tenant, 73002, &draft, &target, &provenance)
            .await
            .unwrap(),
        person_carers::AssignmentOutcome::Assigned
    ));
    tenant.commit().await.unwrap();
    let row = app.fixture.admin.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
        "SELECT carer_id,patient_id,relationship_type FROM carer_relationships WHERE household_id=72001"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "carer_id").unwrap(), 73004);
    assert_eq!(row.try_get::<i64>("", "patient_id").unwrap(), 73002);
    assert_eq!(
        row.try_get::<String>("", "relationship_type").unwrap(),
        "parent"
    );
    app.close().await;
}

#[tokio::test]
async fn person_carer_removal_withdraws_real_health_access_without_removing_manual_grants() {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            health_events::browser,
            person_carers,
        },
    };
    use sea_orm::ConnectionTrait;
    let app = super::Application::new().await;
    parent_assignment_fixture(&app).await;
    app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='administrator' WHERE id=74001; INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at) VALUES(78006,72001,74001,73004,'view','family_member',now(),now())").await.unwrap();
    let manager = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-carer-manager".into(),
    };
    let carer = HouseholdScope {
        actor: Actor { account_id: 71003 },
        household_id: 72001,
        request_id: "synthetic-former-carer".into(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".into(),
    };
    let target = url::Url::parse("http://127.0.0.1/invitations/accept").unwrap();
    let draft = std::collections::HashMap::from([
        ("carer_id".into(), "73004".into()),
        ("relationship_type".into(), "parent".into()),
    ]);
    let tenant = access::begin(&app.fixture.runtime, &manager).await.unwrap();
    person_carers::assign(&tenant, 73002, &draft, &target, &provenance)
        .await
        .unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &carer).await.unwrap();
    assert!(browser::collection(&tenant, 73002).await.is_ok());
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &manager).await.unwrap();
    let page = person_carers::page(&tenant, 73002).await.unwrap();
    let relation = &page["records"][0];
    person_carers::change(
        &tenant,
        73002,
        relation["id"].as_i64().unwrap(),
        false,
        relation["version"].as_str().unwrap(),
        &provenance,
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &carer).await.unwrap();
    assert!(browser::collection(&tenant, 73002).await.is_err());
    assert!(
        browser::save(
            &tenant,
            73002,
            None,
            &std::collections::HashMap::from([
                ("event_kind".into(), "illness".into()),
                ("title".into(), "Forbidden synthetic event".into()),
                ("started_on".into(), "2026-10-01".into())
            ]),
            &provenance
        )
        .await
        .is_err()
    );
    tenant.rollback().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &manager).await.unwrap();
    assert!(browser::collection(&tenant, 73002).await.is_ok());
    tenant.commit().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn person_carer_pending_invitation_cannot_attach_a_dependent_its_original_inviter_cannot_manage()
 {
    use med_tracker::models::{
        access::{self, Actor, HouseholdScope},
        care::{
            doses::{CredentialMethod, CredentialProvenance},
            person_carers,
        },
    };
    use sea_orm::ConnectionTrait;
    let app = super::Application::new().await;
    parent_assignment_fixture(&app).await;
    app.fixture.admin.execute_unprepared(
        "INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87001,72001,'pending-parent@example.test','member','synthetic-unused-invitation-digest',74002,now()+interval '7 days',now(),now())"
    ).await.unwrap();
    let scope = HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-pending-parent-authority".into(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".into(),
    };
    let target = url::Url::parse("http://127.0.0.1/invitations/accept").unwrap();
    let draft =
        std::collections::HashMap::from([("email".into(), "pending-parent@example.test".into())]);
    let tenant = access::begin(&app.fixture.runtime, &scope).await.unwrap();
    let result = person_carers::assign(&tenant, 73002, &draft, &target, &provenance).await;
    tenant.rollback().await.unwrap();
    assert!(
        result.is_err(),
        "A reused invitation must remain acceptable by its original inviter"
    );
    let row=app.fixture.admin.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM household_invitation_grants WHERE household_invitation_id=87001) AS grants,(SELECT count(*) FROM versions WHERE item_type='HouseholdInvitationGrant') AS audits"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "grants").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 0);
    app.close().await;
}
