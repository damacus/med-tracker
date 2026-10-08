use super::fixture::Fixture;
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};

async fn projection(fixture: &Fixture, account: i64, token: &str) -> (i64, Option<i64>, i64) {
    let tx = fixture.runtime.begin().await.unwrap();
    tx.execute_unprepared("SET LOCAL ROLE med_tracker_app").await.unwrap();
    tx.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT set_config('med_tracker.current_account_id',$1,true),set_config('med_tracker.identity_session_token',$2,true),set_config('med_tracker.current_household_id','',true)",
        [account.to_string().into(),token.into()])).await.unwrap();
    let row=tx.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM household_invitations WHERE id=87090) AS invitations,public.identity_invitation_inviter(87090) AS inviter,(SELECT count(*) FROM household_memberships WHERE id=74001) AS memberships")).await.unwrap().unwrap();
    let result=(row.try_get("","invitations").unwrap(),row.try_get("","inviter").unwrap(),row.try_get("","memberships").unwrap());
    tx.rollback().await.unwrap();
    result
}

#[tokio::test]
async fn better_auth_invitation_projection_reveals_only_verified_recipient_inviter_id() {
    let fixture=Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_sessions(id,account_id,token,expires_at,purpose) VALUES('invitation-recipient',71002,'synthetic-recipient-session',now()+interval '1 hour','authenticated'); INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) SELECT 87090,72001,email,'member','synthetic-invitation-digest',74001,now()+interval '1 day',now(),now() FROM accounts WHERE id=71002").await.unwrap();
    assert_eq!(projection(&fixture,71002,"synthetic-recipient-session").await,(1,Some(71001),0));
    let other=projection(&fixture,71001,"synthetic-recipient-session").await;
    assert_eq!((other.0,other.1),(0,None));
    assert_eq!(projection(&fixture,71002,"wrong-session").await,(0,None,0));
    fixture.admin.execute_unprepared("UPDATE identity_sessions SET purpose='enrolment' WHERE id='invitation-recipient'").await.unwrap();
    assert_eq!(projection(&fixture,71002,"synthetic-recipient-session").await,(0,None,0));
    fixture.close().await;
}

#[tokio::test]
async fn better_auth_invitation_projection_denies_expiry_revocation_and_account_lockout() {
    let fixture=Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_sessions(id,account_id,token,expires_at,purpose) VALUES('invitation-recipient',71002,'synthetic-recipient-session',now()+interval '1 hour','authenticated'); INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) SELECT 87090,72001,email,'member','synthetic-invitation-digest',74001,now()+interval '1 day',now(),now() FROM accounts WHERE id=71002").await.unwrap();
    assert_eq!(projection(&fixture,71002,"synthetic-recipient-session").await,(1,Some(71001),0));
    fixture.admin.execute_unprepared("UPDATE household_invitations SET expires_at=now()-interval '1 second' WHERE id=87090").await.unwrap();
    assert_eq!(projection(&fixture,71002,"synthetic-recipient-session").await,(0,None,0));
    fixture.admin.execute_unprepared("UPDATE household_invitations SET expires_at=now()+interval '1 day',revoked_at=now() WHERE id=87090").await.unwrap();
    assert_eq!(projection(&fixture,71002,"synthetic-recipient-session").await,(0,None,0));
    fixture.admin.execute_unprepared("UPDATE household_invitations SET revoked_at=NULL WHERE id=87090; INSERT INTO account_lockouts(account_id,deadline,key) VALUES(71002,now()+interval '1 hour','synthetic-lockout')").await.unwrap();
    assert_eq!(projection(&fixture,71002,"synthetic-recipient-session").await,(0,None,0));
    fixture.close().await;
}
