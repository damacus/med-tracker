use super::{Fixture, assert_database_code, entities};
use migration::sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, IsolationLevel, Statement,
    TransactionTrait,
};

#[tokio::test]
async fn repeatable_read_snapshot_cannot_make_a_newly_linked_person_minor() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(91001,'Adult',now(),now())").await.expect("adult");
    let stale = fixture
        .database
        .begin_with_config(Some(IsolationLevel::RepeatableRead), None)
        .await
        .expect("repeatable read");
    stale
        .execute_unprepared("SELECT * FROM people WHERE id=91001")
        .await
        .expect("establish old snapshot");
    fixture.database.execute_unprepared("INSERT INTO accounts(person_id,name,email,status,created_at,updated_at) VALUES(91001,'Adult profile','snapshot@example.test',2,now(),now())").await.expect("concurrent committed account");
    let mutation = stale
        .execute_unprepared("UPDATE people SET person_type=1,has_capacity=false WHERE id=91001")
        .await;
    let error = match mutation {
        Ok(_) => {
            stale.commit().await.expect("commit stale update");
            None
        }
        Err(error) => {
            stale
                .rollback()
                .await
                .expect("rollback serialization conflict");
            Some(error)
        }
    };
    let row = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT person_type,has_capacity,(SELECT count(*) FROM accounts WHERE person_id=91001) AS accounts FROM people WHERE id=91001"
    )).await.expect("persisted outcome").expect("person");
    fixture.close().await;
    assert_database_code(
        error.expect("Stale snapshot must not commit a minor linked to an account"),
        "40001",
    );
    assert_eq!(row.try_get::<i32>("", "person_type").unwrap(), 0);
    assert!(row.try_get::<bool>("", "has_capacity").unwrap());
    assert_eq!(row.try_get::<i64>("", "accounts").unwrap(), 1);
}

#[tokio::test]
async fn repeatable_read_account_link_cannot_miss_a_committed_minor_edit() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(91101,'Adult',now(),now())").await.expect("adult");
    let stale = fixture
        .database
        .begin_with_config(Some(IsolationLevel::RepeatableRead), None)
        .await
        .expect("repeatable read");
    stale
        .execute_unprepared("SELECT * FROM people WHERE id=91101")
        .await
        .expect("establish old snapshot");
    fixture
        .database
        .execute_unprepared("UPDATE people SET person_type=1,has_capacity=false WHERE id=91101")
        .await
        .expect("committed minor edit");
    let mutation = stale.execute_unprepared("INSERT INTO accounts(person_id,name,email,status,created_at,updated_at) VALUES(91101,'Invalid profile','inverse@example.test',2,now(),now())").await;
    stale.rollback().await.expect("rollback account link");
    let row = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT person_type,has_capacity,(SELECT count(*) FROM accounts WHERE person_id=91101) AS accounts FROM people WHERE id=91101"
    )).await.expect("persisted outcome").expect("person");
    fixture.close().await;
    assert_database_code(
        mutation.expect_err("Old account snapshot must reject changed person"),
        "40001",
    );
    assert_eq!(row.try_get::<i32>("", "person_type").unwrap(), 1);
    assert!(!row.try_get::<bool>("", "has_capacity").unwrap());
    assert_eq!(row.try_get::<i64>("", "accounts").unwrap(), 0);
}

async fn shared_household(fixture: &Fixture) {
    fixture.database.execute_unprepared(
        "INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(91201,'Actor',now(),now()),(91202,'Peer',now(),now()),(91203,'Other person',now(),now());
         INSERT INTO accounts(id,person_id,name,email,status,created_at,updated_at) VALUES(91204,91201,'Actor profile','Actor@Example.test',2,now(),now()),(91205,91202,'Peer profile','Peer@Example.test',2,now(),now());
         INSERT INTO households(id,name,slug,timezone,created_at,updated_at) VALUES(91206,'Home','home','UTC',now(),now()),(91207,'Other','other','UTC',now(),now());
         INSERT INTO household_memberships(id,person_id,household_id,role,joined_at,created_at,updated_at) VALUES(91208,91201,91206,'member',now(),now(),now()),(91209,91202,91206,'member',now(),now(),now());
         INSERT INTO household_invitations(id,household_id,invited_by_membership_id,email,membership_role,token_digest,expires_at,created_at,updated_at) VALUES(91210,91206,91208,'Invitee@Example.test','member','first-token',now()+interval '1 day',now(),now())"
    ).await.expect("shared clinical household");
}

#[tokio::test]
async fn generated_citext_fields_retain_real_account_and_invitation_emails() {
    use migration::sea_orm::{
        ActiveModelTrait, ColumnTrait, IntoActiveModel, NotSet, QueryFilter, Set,
    };
    let fixture = Fixture::new().await;
    shared_household(&fixture).await;
    let account = entities::accounts::Entity::find_by_id(91204)
        .one(&fixture.database)
        .await
        .expect("ORM account read")
        .expect("account");
    let invitation = entities::household_invitations::Entity::find_by_id(91210)
        .one(&fixture.database)
        .await
        .expect("ORM invitation read")
        .expect("invitation");
    assert_eq!(account.email, "Actor@Example.test");
    assert_eq!(invitation.email, "Invitee@Example.test");
    let mut new_account = account.into_active_model();
    new_account.id = NotSet;
    new_account.person_id = Set(91203);
    new_account.email = Set("Second@Example.test".to_owned());
    let created = new_account
        .insert(&fixture.database)
        .await
        .expect("ORM account email insert");
    assert_eq!(created.email, "Second@Example.test");
    let found = entities::accounts::Entity::find()
        .filter(entities::accounts::Column::Email.eq("SECOND@EXAMPLE.TEST"))
        .one(&fixture.database)
        .await
        .expect("case-insensitive account filter")
        .expect("account");
    assert_eq!(found.id, created.id);
    let mut updated = found.into_active_model();
    updated.email = Set("Changed@Example.test".to_owned());
    let changed = updated
        .update(&fixture.database)
        .await
        .expect("ORM account email update");
    let found = entities::accounts::Entity::find()
        .filter(entities::accounts::Column::Email.eq("changed@example.test"))
        .one(&fixture.database)
        .await
        .expect("updated account filter")
        .expect("account");
    assert_eq!(found.email, "Changed@Example.test");
    fixture.database.execute_unprepared("INSERT INTO people(id,clinical_name,created_at,updated_at) VALUES(91213,'Distinct person',now(),now())").await.expect("distinct duplicate account person");
    let mut duplicate = changed.into_active_model();
    duplicate.id = NotSet;
    duplicate.person_id = Set(91213);
    duplicate.email = Set("CHANGED@example.test".to_owned());
    assert_database_code(
        duplicate
            .insert(&fixture.database)
            .await
            .expect_err("case-insensitive email uniqueness"),
        "23505",
    );

    let mut new_invitation = invitation.into_active_model();
    new_invitation.id = NotSet;
    new_invitation.email = Set("SecondInvite@Example.test".to_owned());
    new_invitation.token_digest = Set("second-token".to_owned());
    let created = new_invitation
        .insert(&fixture.database)
        .await
        .expect("ORM invitation email insert");
    assert_eq!(created.email, "SecondInvite@Example.test");
    let found = entities::household_invitations::Entity::find()
        .filter(entities::household_invitations::Column::Email.eq("SECONDINVITE@EXAMPLE.TEST"))
        .one(&fixture.database)
        .await
        .expect("case-insensitive invitation filter")
        .expect("invitation");
    assert_eq!(found.id, created.id);
    let mut updated = found.into_active_model();
    updated.email = Set("ChangedInvite@Example.test".to_owned());
    let changed = updated
        .update(&fixture.database)
        .await
        .expect("ORM invitation email update");
    let found = entities::household_invitations::Entity::find()
        .filter(entities::household_invitations::Column::Email.eq("changedinvite@example.test"))
        .one(&fixture.database)
        .await
        .expect("updated invitation filter")
        .expect("invitation");
    assert_eq!(found.email, "ChangedInvite@Example.test");
    let mut duplicate = changed.into_active_model();
    duplicate.id = NotSet;
    duplicate.token_digest = Set("third-token".to_owned());
    duplicate.email = Set("CHANGEDINVITE@example.test".to_owned());
    assert_database_code(
        duplicate
            .insert(&fixture.database)
            .await
            .expect_err("case-insensitive invitation uniqueness"),
        "23505",
    );
    fixture.close().await;
}

#[tokio::test]
async fn retained_participation_cannot_reassign_actor_identity_or_be_deleted() {
    let fixture = Fixture::new().await;
    shared_household(&fixture).await;
    fixture.database.execute_unprepared(
        "INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(91211,91206,'Cabinet',now(),now());
         INSERT INTO medications(id,household_id,location_id,created_by_membership_id,name,created_at,updated_at) VALUES(91212,91206,91211,91208,'Actor reference',now(),now())"
    ).await.expect("actor reference without any subject-care FK");
    let mut failures = Vec::new();
    for sql in [
        "UPDATE household_memberships SET person_id=91203 WHERE id=91208",
        "UPDATE household_memberships SET id=91309 WHERE id=91209",
        "UPDATE household_memberships SET household_id=91207 WHERE id=91209",
        "DELETE FROM household_memberships WHERE id=91209",
    ] {
        let owner = fixture.database.begin().await.expect("owner transaction");
        owner
            .execute_unprepared("SET LOCAL ROLE med_tracker_owner")
            .await
            .expect("same role as narrow definers");
        let result = owner.execute_unprepared(sql).await;
        owner.rollback().await.expect("isolate invariant check");
        match result {
            Ok(_) => failures.push(sql),
            Err(error) => assert_database_code(error, "23514"),
        }
    }
    let owner = fixture
        .database
        .begin()
        .await
        .expect("owner lifecycle transaction");
    owner.execute_unprepared("SET LOCAL ROLE med_tracker_owner; UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=91209").await.expect("revocation remains supported");
    owner.commit().await.expect("commit revocation");
    let owner = fixture
        .database
        .begin()
        .await
        .expect("retention transaction");
    owner
        .execute_unprepared("SET LOCAL ROLE med_tracker_owner")
        .await
        .expect("owner role");
    let deletion = owner
        .execute_unprepared("DELETE FROM household_memberships WHERE id=91209")
        .await;
    owner.rollback().await.expect("rollback deletion attempt");
    match deletion {
        Ok(_) => failures.push("delete unreferenced revoked participation"),
        Err(error) => assert_database_code(error, "23514"),
    }
    let row = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT membership.person_id,(SELECT status FROM household_memberships WHERE id=91209) AS retained_status FROM medications medication JOIN household_memberships membership ON membership.id=medication.created_by_membership_id AND membership.household_id=medication.household_id WHERE medication.id=91212"
    )).await.expect("retained actor attribution").expect("actor");
    fixture.close().await;
    assert!(
        failures.is_empty(),
        "Retained identity mutations accepted: {failures:?}"
    );
    assert_eq!(row.try_get::<i64>("", "person_id").unwrap(), 91201);
    assert_eq!(
        row.try_get::<String>("", "retained_status").unwrap(),
        "revoked"
    );
}

async fn assert_peer_scope(
    runtime: &DatabaseConnection,
    visible: bool,
    household: bool,
    self_visible: bool,
) {
    let row = runtime
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT med_tracker.current_household_id() IS NOT NULL AS selected,
          (SELECT count(*) FROM public.households WHERE id=91206) AS households,
          (SELECT count(*) FROM public.people WHERE id=91202) AS peers,
          (SELECT count(*) FROM public.people WHERE id=91201) AS selves,
          med_tracker.person_display_name(91202) AS peer_name",
        ))
        .await
        .expect("restricted stale selection query")
        .expect("scope");
    assert_eq!(row.try_get::<bool>("", "selected").unwrap(), household);
    assert_eq!(
        row.try_get::<i64>("", "households").unwrap(),
        i64::from(household)
    );
    assert_eq!(row.try_get::<i64>("", "peers").unwrap(), i64::from(visible));
    assert_eq!(
        row.try_get::<i64>("", "selves").unwrap(),
        i64::from(self_visible)
    );
    assert_eq!(
        row.try_get::<Option<String>>("", "peer_name").unwrap(),
        visible.then(|| "Peer profile".to_owned())
    );
}

#[tokio::test]
async fn restricted_stale_selection_loses_revoked_and_inactive_scope() {
    let fixture = Fixture::new().await;
    shared_household(&fixture).await;
    let (runtime, role) = fixture.runtime().await;
    runtime.execute_unprepared("SET med_tracker.current_account_id='91204'; SET med_tracker.current_household_id='91206'; SET med_tracker.current_membership_id='91208'").await.expect("selected home");
    assert_peer_scope(&runtime, true, true, true).await;
    for (change, restore, household, self_visible) in [
        (
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=91208",
            "UPDATE household_memberships SET status='active',revoked_at=NULL WHERE id=91208",
            false,
            true,
        ),
        (
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=91209",
            "UPDATE household_memberships SET status='active',revoked_at=NULL WHERE id=91209",
            true,
            true,
        ),
        (
            "UPDATE households SET status='inactive' WHERE id=91206",
            "UPDATE households SET status='active' WHERE id=91206",
            false,
            true,
        ),
        (
            "UPDATE households SET lifecycle_state='archived' WHERE id=91206",
            "UPDATE households SET lifecycle_state='active' WHERE id=91206",
            false,
            true,
        ),
        (
            "UPDATE accounts SET status=1 WHERE id=91204",
            "UPDATE accounts SET status=2 WHERE id=91204",
            false,
            true,
        ),
        (
            "UPDATE accounts SET status=3 WHERE id=91204",
            "UPDATE accounts SET status=2 WHERE id=91204",
            false,
            false,
        ),
        (
            "INSERT INTO account_lockouts(account_id,deadline,key) VALUES(91204,now()+interval '1 hour','locked')",
            "DELETE FROM account_lockouts WHERE account_id=91204",
            false,
            true,
        ),
    ] {
        fixture
            .database
            .execute_unprepared(change)
            .await
            .expect("invalidate on independent connection");
        assert_peer_scope(&runtime, false, household, self_visible).await;
        if change.contains("WHERE id=91209") {
            runtime.execute_unprepared("SET med_tracker.current_account_id='91205'; SET med_tracker.current_membership_id='91209'").await.expect("revoked target authenticated identity");
            let row = runtime
                .query_one_raw(Statement::from_string(
                    DbBackend::Postgres,
                    "SELECT (SELECT count(*) FROM people WHERE id=91202) AS self_count,
                  (SELECT count(*) FROM people WHERE id=91201) AS peer_count,
                  med_tracker.current_household_id() AS household,
                  med_tracker.person_display_name(91202) AS name",
                ))
                .await
                .expect("revoked target self scope")
                .expect("scope");
            assert_eq!(row.try_get::<i64>("", "self_count").unwrap(), 1);
            assert_eq!(row.try_get::<i64>("", "peer_count").unwrap(), 0);
            assert_eq!(row.try_get::<Option<i64>>("", "household").unwrap(), None);
            assert_eq!(row.try_get::<String>("", "name").unwrap(), "Peer profile");
            runtime.execute_unprepared("SET med_tracker.current_account_id='91204'; SET med_tracker.current_membership_id='91208'").await.expect("restore actor identity");
        }
        fixture
            .database
            .execute_unprepared(restore)
            .await
            .expect("restore independent case");
        assert_peer_scope(&runtime, true, true, true).await;
    }
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
}

#[tokio::test]
async fn identity_helpers_have_minimal_execution_grants_and_fixed_owner_search_path() {
    let fixture = Fixture::new().await;
    let (runtime, role) = fixture.runtime().await;
    for function in [
        "enforce_account_person",
        "enforce_person_account",
        "enforce_participation_identity",
        "preserve_clinical_name",
        "require_person_name",
    ] {
        let error = runtime
            .execute_unprepared(&format!("SELECT med_tracker.{function}()"))
            .await
            .expect_err("restricted login cannot execute trigger-only helper");
        assert_database_code(error, "42501");
    }
    assert_database_code(
        runtime
            .execute_unprepared("SET ROLE med_tracker_owner")
            .await
            .expect_err("runtime cannot become helper owner"),
        "42501",
    );
    fixture.close_runtime(runtime, &role).await;
    let owner = fixture.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT rolcanlogin,rolsuper,rolcreatedb,rolcreaterole,rolreplication,rolbypassrls FROM pg_roles WHERE rolname='med_tracker_owner'"
    )).await.expect("helper owner role metadata").expect("owner");
    for privilege in [
        "rolcanlogin",
        "rolsuper",
        "rolcreatedb",
        "rolcreaterole",
        "rolreplication",
        "rolbypassrls",
    ] {
        assert!(
            !owner.try_get::<bool>("", privilege).unwrap(),
            "Helper owner has {privilege}"
        );
    }
    let rows = fixture.database.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT procedure.proname, pg_get_userbyid(procedure.proowner) AS owner,procedure.prosecdef,
          array_to_string(procedure.proconfig,',') AS configuration,
          EXISTS(SELECT 1 FROM aclexplode(COALESCE(procedure.proacl,acldefault('f',procedure.proowner))) acl WHERE acl.grantee=0 AND acl.privilege_type='EXECUTE') AS public_execution,
          has_function_privilege('med_tracker_app',procedure.oid,'EXECUTE') AS app_execution
         FROM pg_proc procedure JOIN pg_namespace namespace ON namespace.oid=procedure.pronamespace
         WHERE namespace.nspname='med_tracker' AND procedure.proname IN
          ('record_low_stock_notifications','reminder_recipients','cleanup_reminders','current_person_id','current_household_id','has_household_access','can_read_person','person_display_name','person_avatar_digest','person_avatar','avatar_key_retained','household_accounts','invitation_historical_inviter','platform_accounts','platform_recovery_households','platform_recovery_members','check_household_owner_recovery','recover_household_owner','platform_account_state','set_platform_account_active','bootstrap_account','bootstrap_registration','create_household','create_household_person','update_clinical_person','lock_current_household','change_household_membership','update_household_settings','can_edit_clinical_person','accept_household_invitation','invitation_preview','complete_household_invitation','bootstrap_invited_registration','enforce_account_person','enforce_person_account','enforce_participation_identity','preserve_clinical_name','require_person_name') ORDER BY procedure.proname"
    )).await.expect("actual helper ACL metadata");
    fixture.close().await;
    assert_eq!(rows.len(), 38);
    let mut violations = Vec::new();
    for row in rows {
        let name: String = row.try_get("", "proname").unwrap();
        assert_eq!(
            row.try_get::<String>("", "owner").unwrap(),
            "med_tracker_owner"
        );
        assert!(row.try_get::<bool>("", "prosecdef").unwrap());
        assert_eq!(
            row.try_get::<String>("", "configuration").unwrap(),
            "search_path=pg_catalog"
        );
        if row.try_get::<bool>("", "public_execution").unwrap() {
            violations.push(format!("PUBLIC can execute {name}"));
        }
        let trigger = [
            "enforce_account_person",
            "enforce_person_account",
            "enforce_participation_identity",
            "preserve_clinical_name",
            "require_person_name",
        ]
        .contains(&name.as_str())
            || name == "complete_household_invitation";
        if row.try_get::<bool>("", "app_execution").unwrap() == trigger {
            violations.push(format!("Incorrect app execution for {name}"));
        }
    }
    assert!(violations.is_empty(), "Helper privileges: {violations:?}");
}
