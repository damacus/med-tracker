use loco_rs::{controller::views::ViewRenderer, prelude::TeraView};
use sea_orm::IntoActiveModel as _;
use sea_orm::sea_query::{Expr as QueryExpr, ExprTrait as _, Func as QueryFunc, Query};
use sea_orm::{ActiveModelTrait as _, ColumnTrait as _, EntityTrait as _, QueryFilter as _};
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
                    "labels":labels,"carer_labels":labels["people"]["carer_relationships"],
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

async fn parent_assignment_fixture(
    app: &super::Application,
) -> (
    med_tracker::models::_entities::person_access_grants::Model,
    med_tracker::models::_entities::people::Model,
    med_tracker::models::_entities::accounts::Model,
    med_tracker::models::_entities::household_memberships::Model,
) {
    let candidate_email = format!("parent-candidate+{}@example.test", uuid::Uuid::new_v4());
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Role,
            sea_orm::sea_query::Expr::value("member".to_owned()),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::Id
                .eq(fixture_membership_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _person_access_grants_78005 =
        med_tracker::models::_entities::person_access_grants::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            household_membership_id: sea_orm::Set(fixture_membership_id),
            person_id: sea_orm::Set(fixture_minor_id),
            access_level: sea_orm::Set("manage".to_owned()),
            relationship_type: sea_orm::Set("parent".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _people_73004 = med_tracker::models::_entities::people::ActiveModel {
        name: sea_orm::Set("Synthetic candidate".to_owned()),
        person_type: sea_orm::Set(0),
        has_capacity: sea_orm::Set(true),
        created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _accounts_71003 = med_tracker::models::_entities::accounts::ActiveModel {
        person_id: sea_orm::Set(_people_73004.id),
        name: sea_orm::Set(Some("Synthetic candidate".to_owned())),
        email: sea_orm::Set(candidate_email.clone()),
        status: sea_orm::Set(2),
        created_at: sea_orm::Set(chrono::Utc::now()),
        updated_at: sea_orm::Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let _household_memberships_74002 =
        med_tracker::models::_entities::household_memberships::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            person_id: sea_orm::Set(_people_73004.id),
            role: sea_orm::Set("member".to_owned()),
            status: sea_orm::Set("active".to_owned()),
            joined_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();

    (
        _person_access_grants_78005,
        _people_73004,
        _accounts_71003,
        _household_memberships_74002,
    )
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
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let (_person_access_grants_78005, _people_73004, _accounts_71003, _household_memberships_74002) =
        parent_assignment_fixture(&app).await;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-parent-restrictions".to_owned(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".to_owned(),
    };
    let target = url::Url::parse("http://127.0.0.1/invitations/accept").unwrap();
    let draft = std::collections::HashMap::from([
        ("email".to_owned(), _accounts_71003.email.clone()),
        (
            "relationship_type".to_owned(),
            "professional_carer".to_owned(),
        ),
    ]);
    let mut self_relationship: Option<med_tracker::models::_entities::carer_relationships::Model> =
        None;
    let mut inactive_relationship: Option<
        med_tracker::models::_entities::carer_relationships::Model,
    > = None;
    for mutation in 0..9 {
        match mutation {
            0 => {
                let mut model = _people_73004.clone().into_active_model();
                model.professional_title = sea_orm::Set(Some("Clinician".into()));
                model.update(&app.fixture.admin).await.unwrap();
            }
            1 => {
                let mut model = _people_73004.clone().into_active_model();
                model.professional_title = sea_orm::Set(None);
                model.has_capacity = sea_orm::Set(false);
                model.update(&app.fixture.admin).await.unwrap();
            }
            2 => {
                let mut model = _people_73004.clone().into_active_model();
                model.has_capacity = sea_orm::Set(true);
                model.person_type = sea_orm::Set(1);
                model.update(&app.fixture.admin).await.unwrap();
            }
            3 => {
                {
                    let mut model = _people_73004.clone().into_active_model();
                    model.person_type = sea_orm::Set(0);
                    model.update(&app.fixture.admin).await.unwrap();
                }
                {
                    let mut model = _accounts_71003.clone().into_active_model();
                    model.status = sea_orm::Set(1);
                    model.update(&app.fixture.admin).await.unwrap();
                }
            }
            4 => {
                {
                    let mut model = _accounts_71003.clone().into_active_model();
                    model.status = sea_orm::Set(2);
                    model.update(&app.fixture.admin).await.unwrap();
                }
                {
                    let mut model = _accounts_71003.clone().into_active_model();
                    model.active = sea_orm::Set(false);
                    model.update(&app.fixture.admin).await.unwrap();
                }
            }
            5 => {
                {
                    let mut model = _accounts_71003.clone().into_active_model();
                    model.active = sea_orm::Set(true);
                    model.update(&app.fixture.admin).await.unwrap();
                }
                {
                    let mut model = _household_memberships_74002.clone().into_active_model();
                    model.revoked_at = sea_orm::Set(Some(chrono::Utc::now().naive_utc()));
                    model.update(&app.fixture.admin).await.unwrap();
                }
            }
            6 => {
                let mut model = _household_memberships_74002.clone().into_active_model();
                model.revoked_at = sea_orm::Set(None);
                model.status = sea_orm::Set("suspended".into());
                model.update(&app.fixture.admin).await.unwrap();
            }
            7 => {
                {
                    let mut model = _household_memberships_74002.clone().into_active_model();
                    model.status = sea_orm::Set("active".into());
                    model.update(&app.fixture.admin).await.unwrap();
                }
                self_relationship = Some(
                    med_tracker::models::_entities::carer_relationships::ActiveModel {
                        household_id: sea_orm::Set(fixture_household_id),
                        carer_id: sea_orm::Set(_people_73004.id),
                        patient_id: sea_orm::Set(_people_73004.id),
                        relationship_type: sea_orm::Set(Some("self".into())),
                        active: sea_orm::Set(true),
                        created_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                        updated_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                        ..Default::default()
                    }
                    .insert(&app.fixture.admin)
                    .await
                    .unwrap(),
                );
            }
            8 => {
                med_tracker::models::_entities::carer_relationships::Entity::delete_by_id(
                    self_relationship.take().unwrap().id,
                )
                .exec(&app.fixture.admin)
                .await
                .unwrap();
                inactive_relationship = Some(
                    med_tracker::models::_entities::carer_relationships::ActiveModel {
                        household_id: sea_orm::Set(fixture_household_id),
                        carer_id: sea_orm::Set(_people_73004.id),
                        patient_id: sea_orm::Set(fixture_minor_id),
                        relationship_type: sea_orm::Set(Some("parent".into())),
                        active: sea_orm::Set(false),
                        created_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                        updated_at: sea_orm::Set(chrono::Utc::now().naive_utc()),
                        ..Default::default()
                    }
                    .insert(&app.fixture.admin)
                    .await
                    .unwrap(),
                );
            }
            _ => unreachable!(),
        }
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        let result =
            person_carers::assign(&tenant, fixture_minor_id, &draft, &target, &provenance).await;
        tenant.rollback().await.unwrap();
        assert!(result.is_err(), "{mutation}");
    }
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert!(
        person_carers::change(
            &tenant,
            fixture_minor_id,
            inactive_relationship.as_ref().unwrap().id,
            true,
            "",
            &provenance
        )
        .await
        .is_err()
    );
    tenant.rollback().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("carer_relationships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("active")),
                            )
                            .to_owned(),
                    ),
                    "active",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("CarerRelationship")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "audits",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
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
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_person_id = app.fixture.account.person_id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_dependent_id = app.fixture.dependent.id;
    let (_person_access_grants_78005, _people_73004, _accounts_71003, _household_memberships_74002) =
        parent_assignment_fixture(&app).await;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-parent-scope".to_owned(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".to_owned(),
    };
    let target = url::Url::parse("http://127.0.0.1/invitations/accept").unwrap();
    let draft = std::collections::HashMap::from([
        ("email".to_owned(), _accounts_71003.email.clone()),
        (
            "relationship_type".to_owned(),
            "professional_carer".to_owned(),
        ),
        ("carer_id".to_owned(), fixture_person_id.to_string()),
    ]);
    for person_id in [fixture_person_id, fixture_dependent_id, 999999] {
        let tenant = access::begin(&app.fixture.runtime, &scope, None)
            .await
            .unwrap();
        assert!(
            person_carers::assign(&tenant, person_id, &draft, &target, &provenance)
                .await
                .is_err()
        );
        tenant.rollback().await.unwrap();
    }
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    assert!(
        person_carers::assign(&tenant, fixture_minor_id, &draft, &target, &provenance)
            .await
            .is_err()
    );
    tenant.rollback().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("carer_relationships")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("household_id")
                                            .eq(QueryExpr::val(fixture_household_id)),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "relationships",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("person_access_grants")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("carer_relationship_id").is_not_null()),
                            )
                            .to_owned(),
                    ),
                    "grants",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("CarerRelationship")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "versions",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "relationships").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "grants").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "versions").unwrap(), 0);
    app.close().await;
}

#[tokio::test]
async fn person_carer_removal_withdraws_linked_manage_but_preserves_member_record_and_manual_grants()
 {
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
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_membership_id = app.fixture.membership.id;
    let (_person_access_grants_78005, _people_73004, _accounts_71003, _household_memberships_74002) =
        parent_assignment_fixture(&app).await;
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Role,
            sea_orm::sea_query::Expr::value("administrator".to_owned()),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::Id
                .eq(fixture_membership_id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let _person_access_grants_78006 =
        med_tracker::models::_entities::person_access_grants::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            household_membership_id: sea_orm::Set(fixture_membership_id),
            person_id: sea_orm::Set(_people_73004.id),
            access_level: sea_orm::Set("view".to_owned()),
            relationship_type: sea_orm::Set("family_member".to_owned()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let carer_relationships_86001 =
        med_tracker::models::_entities::carer_relationships::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            carer_id: sea_orm::Set(_people_73004.id),
            patient_id: sea_orm::Set(fixture_minor_id),
            relationship_type: sea_orm::Set(Some("parent".to_owned())),
            active: sea_orm::Set(true),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let _person_access_grants_78007 =
        med_tracker::models::_entities::person_access_grants::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            household_membership_id: sea_orm::Set(_household_memberships_74002.id),
            person_id: sea_orm::Set(fixture_minor_id),
            access_level: sea_orm::Set("manage".to_owned()),
            relationship_type: sea_orm::Set("parent".to_owned()),
            carer_relationship_id: sea_orm::Set(Some(carer_relationships_86001.id)),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let manager = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-carer-manager".to_owned(),
    };
    let carer = HouseholdScope {
        actor: Actor {
            account_id: _accounts_71003.id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-former-carer".to_owned(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".to_owned(),
    };
    let tenant = access::begin(&app.fixture.runtime, &carer, None)
        .await
        .unwrap();
    assert!(browser::collection(&tenant, fixture_minor_id).await.is_ok());
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &manager, None)
        .await
        .unwrap();
    let page = person_carers::page(&tenant, fixture_minor_id)
        .await
        .unwrap();
    let relation = &page["records"][0];
    person_carers::change(
        &tenant,
        fixture_minor_id,
        relation["id"].as_i64().unwrap(),
        false,
        relation["version"].as_str().unwrap(),
        &provenance,
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &carer, None)
        .await
        .unwrap();
    assert!(browser::collection(&tenant, fixture_minor_id).await.is_ok());
    assert!(
        access::require_person_access(&tenant, fixture_minor_id, access::PersonAccess::Manage)
            .await
            .is_err()
    );
    assert!(
        browser::save(
            &tenant,
            fixture_minor_id,
            None,
            &std::collections::HashMap::from([
                ("event_kind".to_owned(), "illness".to_owned()),
                (
                    "title".to_owned(),
                    "Synthetic member-record event".to_owned()
                ),
                ("started_on".to_owned(), "2026-10-01".to_owned())
            ]),
            &provenance
        )
        .await
        .is_ok()
    );
    tenant.rollback().await.unwrap();
    let tenant = access::begin(&app.fixture.runtime, &manager, None)
        .await
        .unwrap();
    assert!(browser::collection(&tenant, fixture_minor_id).await.is_ok());
    tenant.commit().await.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("person_access_grants")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(QueryExpr::col("id").is_in([
                                        QueryExpr::val(_person_access_grants_78005.id),
                                        QueryExpr::val(_person_access_grants_78006.id),
                                    ]))
                                    .and(QueryExpr::col("revoked_at").is_null()),
                            )
                            .to_owned(),
                    ),
                    "manual",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("person_access_grants")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("id")
                                            .eq(QueryExpr::val(_person_access_grants_78007.id)),
                                    )
                                    .and(QueryExpr::col("revoked_at").is_not_null()),
                            )
                            .to_owned(),
                    ),
                    "withdrawn",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "manual").unwrap(), 2);
    assert_eq!(row.try_get::<i64>("", "withdrawn").unwrap(), 1);
    med_tracker::models::_entities::household_memberships::Entity::update_many()
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::Status,
            sea_orm::sea_query::Expr::value("revoked".to_owned()),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Some((chrono::Utc::now()).naive_utc())),
        )
        .col_expr(
            med_tracker::models::_entities::household_memberships::Column::PermissionsVersion,
            sea_orm::sea_query::Expr::col(
                med_tracker::models::_entities::household_memberships::Column::PermissionsVersion,
            )
            .add(1),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::Id
                .eq(_household_memberships_74002.id),
        )
        .filter(
            med_tracker::models::_entities::household_memberships::Column::HouseholdId
                .eq(app.fixture.household.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert!(
        access::begin(&app.fixture.runtime, &carer, None)
            .await
            .is_err()
    );
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
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_minor_id = app.fixture.minor.id;
    let (_person_access_grants_78005, _people_73004, _accounts_71003, _household_memberships_74002) =
        parent_assignment_fixture(&app).await;
    let household_invitations_87001 =
        med_tracker::models::_entities::household_invitations::ActiveModel {
            household_id: sea_orm::Set(fixture_household_id),
            email: sea_orm::Set(format!(
                "pending-parent+{}@example.test",
                uuid::Uuid::new_v4()
            )),
            membership_role: sea_orm::Set("member".to_owned()),
            token_digest: sea_orm::Set(uuid::Uuid::new_v4().to_string()),
            invited_by_membership_id: sea_orm::Set(_household_memberships_74002.id),
            expires_at: sea_orm::Set((chrono::Utc::now() + chrono::Duration::days(7)).naive_utc()),
            created_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            updated_at: sea_orm::Set((chrono::Utc::now()).naive_utc()),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    let scope = HouseholdScope {
        actor: Actor {
            account_id: fixture_account_id,
        },
        household_id: fixture_household_id,
        request_id: "synthetic-pending-parent-authority".to_owned(),
    };
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".to_owned(),
    };
    let target = url::Url::parse("http://127.0.0.1/invitations/accept").unwrap();
    let draft = std::collections::HashMap::from([(
        "email".to_owned(),
        household_invitations_87001.email.clone(),
    )]);
    let tenant = access::begin(&app.fixture.runtime, &scope, None)
        .await
        .unwrap();
    let result =
        person_carers::assign(&tenant, fixture_minor_id, &draft, &target, &provenance).await;
    tenant.rollback().await.unwrap();
    assert!(
        result.is_err(),
        "A reused invitation must remain acceptable by its original inviter"
    );
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("household_invitation_grants")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("household_invitation_id")
                                            .eq(QueryExpr::val(household_invitations_87001.id)),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "grants",
                )
                .expr_as(
                    QueryExpr::expr(
                        Query::select()
                            .expr(QueryExpr::expr(QueryFunc::count(
                                sea_orm::sea_query::Expr::col(sea_orm::sea_query::Asterisk),
                            )))
                            .from("versions")
                            .and_where(
                                QueryExpr::col("household_id")
                                    .eq(QueryExpr::val(fixture_household_id))
                                    .and(
                                        QueryExpr::col("item_type")
                                            .eq(QueryExpr::val("HouseholdInvitationGrant")),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "audits",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "grants").unwrap(), 0);
    assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 0);
    app.close().await;
}
