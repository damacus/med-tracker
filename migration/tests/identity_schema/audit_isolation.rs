use super::{Fixture, assert_database_code};
use migration::sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use serde_json::{Value, json};

async fn visible_versions(runtime: &DatabaseConnection) -> Result<Value, DbErr> {
    let row = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT COALESCE(jsonb_agg(item_type || ':' || item_id::text ORDER BY id), '[]'::jsonb) AS visible FROM public.versions",
        ))
        .await
        ?
        .ok_or_else(|| DbErr::Custom("missing version aggregate".into()))?;
    row.try_get("", "visible")
}

async fn seeded_fixture() -> Result<Fixture, DbErr> {
    let fixture = Fixture::new().await;
    let seeded = fixture.database.execute_unprepared(
        "INSERT INTO public.people(id,clinical_name,created_at,updated_at) VALUES
           (95301,'First actor',now(),now()),(95302,'Second actor',now(),now());
         INSERT INTO public.accounts(id,person_id,name,email,status,created_at,updated_at) VALUES
           (95311,95301,'First actor','audit-first@example.test',2,now(),now()),
           (95312,95302,'Second actor','audit-second@example.test',2,now(),now());
         INSERT INTO public.households(id,name,slug,timezone,created_at,updated_at) VALUES
           (95321,'First home','audit-first-home','UTC',now(),now()),
           (95322,'Second home','audit-second-home','UTC',now(),now());
         INSERT INTO public.household_memberships(id,person_id,household_id,role,status,joined_at,created_at,updated_at) VALUES
           (95331,95301,95321,'member','active',now(),now(),now()),
           (95332,95302,95322,'member','active',now(),now(),now()),
           (95333,95302,95321,'member','active',now(),now(),now());
         INSERT INTO public.versions(item_type,item_id,event,household_id,actor_membership_id,whodunnit,audit_context,created_at) VALUES
           ('HealthEvent',95341,'create',95321,95331,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now()),
           ('HealthEvent',95342,'create',95322,95332,'95302','{\"actor_account_id\":95312,\"actor_person_id\":95302}',now()),
           ('HealthEvent',95344,'create',95321,95333,'95302','{\"actor_account_id\":95312,\"actor_person_id\":95302}',now()),
           ('HealthEvent',95345,'create',95321,95331,'95302','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now()),
           ('HealthEvent',95346,'create',95321,95331,'95301','{\"actor_account_id\":95312,\"actor_person_id\":95301}',now()),
           ('MedicationStockRemoval',95343,'stock_removal',95321,95331,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now()),
           ('MedicationStockRemoval',95347,'stock_removal',95321,95333,'95302','{\"actor_account_id\":95312,\"actor_person_id\":95302}',now()),
           ('Account',95311,'update',95321,95331,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now()),
           ('Account',95312,'update',95321,95333,'95302','{\"actor_account_id\":95312,\"actor_person_id\":95302}',now()),
           ('Account',95311,'update',NULL,NULL,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now()),
           ('Account',95312,'update',NULL,NULL,'95302','{\"actor_account_id\":95312,\"actor_person_id\":95302}',now()),
           ('Account',95311,'update',NULL,NULL,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95302}',now()),
           ('Account',95399,'update',NULL,NULL,NULL,'{}',now())"
    ).await;
    if let Err(error) = seeded {
        fixture.close().await;
        return Err(error);
    }
    Ok(fixture)
}

#[tokio::test]
async fn versions_restrict_household_and_global_history_to_current_authority() -> Result<(), DbErr>
{
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let results = async {
    let anonymous = visible_versions(&runtime).await?;
    runtime.execute_unprepared("SET med_tracker.current_account_id='95311'; SET med_tracker.current_household_id='95321'; SET med_tracker.current_membership_id='95331'").await?;
    let first = visible_versions(&runtime).await?;
    runtime
        .execute_unprepared("SET med_tracker.current_membership_id='95333'")
        .await?;
    let forged_membership = visible_versions(&runtime).await?;
    runtime
        .execute_unprepared("SET med_tracker.current_membership_id='95331'")
        .await?;
    fixture
        .database
        .execute_unprepared(
            "UPDATE public.household_memberships SET role='administrator' WHERE id=95331",
        )
        .await?;
    let manager = visible_versions(&runtime).await?;
    runtime
        .execute_unprepared("SET med_tracker.current_household_id='95322'")
        .await?;
    let foreign = visible_versions(&runtime).await?;
    runtime
        .execute_unprepared("SET med_tracker.current_account_id='95312'; SET med_tracker.current_membership_id='95332'")
        .await?;
    let second = visible_versions(&runtime).await?;
    fixture.database.execute_unprepared("UPDATE public.household_memberships SET status='revoked',revoked_at=now() WHERE id=95332").await?;
    let revoked = visible_versions(&runtime).await?;
    Ok::<_, DbErr>((anonymous, first, forged_membership, manager, foreign, second, revoked))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (anonymous, first, forged_membership, manager, foreign, second, revoked) = results?;
    assert_eq!(anonymous, json!([]));
    assert_eq!(
        first,
        json!(["HealthEvent:95341", "Account:95311", "Account:95311"])
    );
    assert_eq!(forged_membership, json!(["Account:95311"]));
    assert_eq!(
        manager,
        json!([
            "HealthEvent:95341",
            "MedicationStockRemoval:95343",
            "MedicationStockRemoval:95347",
            "Account:95311",
            "Account:95311"
        ])
    );
    assert_eq!(foreign, json!(["Account:95311"]));
    assert_eq!(second, json!(["HealthEvent:95342", "Account:95312"]));
    assert_eq!(revoked, json!(["Account:95312"]));
    Ok(())
}

#[tokio::test]
async fn versions_bind_restricted_writes_and_preserve_original_ledger_rows() -> Result<(), DbErr> {
    let fixture = seeded_fixture().await?;
    let (runtime, role) = fixture.runtime().await;
    let results = async {
    runtime.execute_unprepared("SET med_tracker.current_account_id='95311'; SET med_tracker.current_household_id='95321'; SET med_tracker.current_membership_id='95331'").await?;
    runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,"INSERT INTO public.versions(item_type,item_id,event,household_id,actor_membership_id,whodunnit,audit_context,created_at) VALUES ('Medication',95371,'update',95321,95331,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95301,\"actor_membership_id\":95331,\"household_id\":95321}',now()) RETURNING *")).await?.ok_or_else(|| DbErr::Custom("missing household version".into()))?;
    runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,"INSERT INTO public.versions(item_type,item_id,event,whodunnit,audit_context,created_at) VALUES ('AuthenticationToken',95311,'auth_token/session/revoked','95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now()) RETURNING *")).await?.ok_or_else(|| DbErr::Custom("missing token version".into()))?;
    runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,"INSERT INTO public.versions(item_type,item_id,event,whodunnit,audit_context,created_at) VALUES ('Account',95311,'update','95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now()) RETURNING *")).await?.ok_or_else(|| DbErr::Custom("missing account version".into()))?;
    let foreign_household = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,household_id,actor_membership_id,whodunnit,audit_context,created_at) VALUES ('Medication',95372,'update',95322,95332,'95301','{\"actor_account_id\":95311}',now())").await;
    let foreign_global = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,whodunnit,audit_context,created_at) VALUES ('Account',95312,'update','95301','{\"actor_account_id\":95311}',now())").await;
    let actorless_global = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,audit_context,created_at) VALUES ('Account',95311,'update','{}',now())").await;
    let unrelated_global_event = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,whodunnit,audit_context,created_at) VALUES ('Account',95311,'arbitrary','95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now())").await;
    let global_with_membership = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,actor_membership_id,whodunnit,audit_context,created_at) VALUES ('AuthenticationToken',95311,'auth_token/session/created',95331,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95301,\"household_id\":95321}',now())").await;
    let forged_actor = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,household_id,actor_membership_id,whodunnit,audit_context,created_at) VALUES ('Medication',95373,'update',95321,95331,'95301','{\"actor_account_id\":95312}',now())").await;
    let forged_membership = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,household_id,actor_membership_id,whodunnit,audit_context,created_at) VALUES ('Medication',95374,'update',95321,95332,'95301','{\"actor_account_id\":95311}',now())").await;
    let peer_account_household = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,household_id,actor_membership_id,whodunnit,audit_context,created_at) VALUES ('Account',95312,'update',95321,95331,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now())").await;
    let peer_token_household = runtime.execute_unprepared("INSERT INTO public.versions(item_type,item_id,event,household_id,actor_membership_id,whodunnit,audit_context,created_at) VALUES ('AuthenticationToken',95312,'auth_token/session/revoked',95321,95331,'95301','{\"actor_account_id\":95311,\"actor_person_id\":95301}',now())").await;
    let update = runtime
        .execute_unprepared("UPDATE public.versions SET event='erased' WHERE item_id=95371")
        .await;
    let delete = runtime
        .execute_unprepared("DELETE FROM public.versions WHERE item_id=95371")
        .await;
    let source = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT source_payload FROM public.audit_ledger_entries WHERE source_table='versions' AND source_payload->>'item_id'='95371'"
    )).await?.ok_or_else(|| DbErr::Custom("missing household ledger entry".into()))?;
    let source: Value = source.try_get("", "source_payload")?;
    let count = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS count FROM public.audit_ledger_entries WHERE source_table='versions'"
    )).await?.ok_or_else(|| DbErr::Custom("missing ledger aggregate".into()))?.try_get::<i64>("", "count")?;
    let verifier_role = format!("audit_verifier_{}", uuid::Uuid::new_v4().simple());
    fixture.admin.execute_unprepared(&format!("CREATE ROLE {verifier_role} LOGIN PASSWORD 'test-password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_audit_verifier")).await?;
    let mut verifier_url = fixture.url.clone();
    verifier_url
        .set_username(&verifier_role)
        .map_err(|()| DbErr::Custom("invalid verifier username".into()))?;
    verifier_url
        .set_password(Some("test-password"))
        .map_err(|()| DbErr::Custom("invalid verifier password".into()))?;
    let verifier = migration::sea_orm::Database::connect(verifier_url.as_str())
        .await?;
    let verifier_count = verifier
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM public.versions",
        ))
        .await;
    verifier.close().await?;
    fixture
        .admin
        .execute_unprepared(&format!("DROP ROLE {verifier_role}"))
        .await?;
    let visible_to_verifier = verifier_count?.ok_or_else(|| DbErr::Custom("missing verifier aggregate".into()))?.try_get::<i64>("", "count")?;
    Ok::<_, DbErr>((foreign_household, foreign_global, actorless_global, unrelated_global_event,
        global_with_membership, forged_actor, forged_membership, peer_account_household,
        peer_token_household, update, delete, source, count, visible_to_verifier))
    }.await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    let (
        foreign_household,
        foreign_global,
        actorless_global,
        unrelated_global_event,
        global_with_membership,
        forged_actor,
        forged_membership,
        peer_account_household,
        peer_token_household,
        update,
        delete,
        source,
        count,
        visible_to_verifier,
    ) = results?;
    assert_database_code(
        foreign_household.expect_err("cross-household write denied"),
        "42501",
    );
    assert_database_code(
        foreign_global.expect_err("foreign global account denied"),
        "42501",
    );
    assert_database_code(
        actorless_global.expect_err("actorless global row denied"),
        "42501",
    );
    assert_database_code(
        unrelated_global_event.expect_err("unrelated global event denied"),
        "42501",
    );
    assert_database_code(
        global_with_membership.expect_err("global event with household attribution denied"),
        "42501",
    );
    assert_database_code(
        forged_actor.expect_err("forged actor context denied"),
        "42501",
    );
    assert_database_code(
        forged_membership.expect_err("forged membership denied"),
        "42501",
    );
    assert_database_code(
        peer_account_household.expect_err("peer account household version denied"),
        "42501",
    );
    assert_database_code(
        peer_token_household.expect_err("peer token household version denied"),
        "42501",
    );
    assert_database_code(update.expect_err("audit update denied"), "42501");
    assert_database_code(delete.expect_err("audit delete denied"), "42501");
    assert_eq!(source["item_id"], 95371);
    assert_eq!(source["whodunnit"], "95301");
    assert_eq!(source["audit_context"]["actor_account_id"], 95311);
    assert_eq!(source["audit_context"]["actor_membership_id"], 95331);
    assert_eq!(count, 16);
    assert_eq!(visible_to_verifier, 16);
    Ok(())
}
