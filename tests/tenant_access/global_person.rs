use super::{Fixture, scope};
use med_tracker::models::{access, errors::OperationError};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

fn require(condition: bool, message: &str) -> Result<(), DbErr> {
    if condition {
        Ok(())
    } else {
        Err(DbErr::Custom(message.to_owned()))
    }
}

async fn account_count(db: &impl ConnectionTrait) -> Result<i64, DbErr> {
    db.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT count(*) AS count FROM public.accounts",
    ))
    .await?
    .ok_or_else(|| DbErr::Custom("Expected Account count".to_owned()))?
    .try_get("", "count")
}

async fn visible(db: &impl ConnectionTrait, person_id: i64) -> Result<bool, DbErr> {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT EXISTS(SELECT 1 FROM public.people WHERE id=$1) AS visible",
        [person_id.into()],
    ))
    .await?
    .ok_or_else(|| DbErr::Custom("Expected subject visibility".to_owned()))?
    .try_get("", "visible")
}

#[tokio::test]
async fn accountless_person_participates_in_two_households_without_credentials() -> Result<(), DbErr>
{
    let fixture = Fixture::new().await;
    let result = async {
        fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET role='member',
             permissions_version=permissions_version+1 WHERE id=74001;
             INSERT INTO public.people(id,name,person_type,has_capacity,created_at,updated_at)
             VALUES(73101,'Synthetic foreign adult',0,true,now(),now()),
                   (73102,'Synthetic foreign dependent',2,false,now(),now());
             INSERT INTO public.accounts(id,person_id,email,status,created_at,updated_at)
             VALUES(71003,73101,'synthetic-third-household@example.test',2,now(),now());
             INSERT INTO public.households(id,name,slug,timezone,created_at,updated_at)
             VALUES(72003,'Synthetic third household','synthetic-third-household','UTC',now(),now());
             INSERT INTO public.household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at)
             VALUES(74503,72003,73101,'owner',now(),now(),now()),
                   (74504,72003,73102,'member',now(),now(),now())"
        ).await?;
        let accounts_before = account_count(&fixture.admin).await?;
        let operation = fixture.admin.execute_unprepared(
            "INSERT INTO public.household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at)
             VALUES (74501,72001,73003,'member',now(),now(),now()),
                    (74502,72002,73003,'member',now(),now(),now())"
        ).await;
        operation?;
        require(account_count(&fixture.admin).await? == accounts_before,
            "Dependent participation must not create an Account")?;
        let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.people WHERE id=73003) AS people,
                    (SELECT count(*) FROM public.household_memberships WHERE person_id=73003
                     AND status='active' AND revoked_at IS NULL) AS memberships,
                    (SELECT count(*) FROM public.users WHERE person_id=73003) AS users"))
            .await?.ok_or_else(|| DbErr::Custom("Expected dependent identity state".to_owned()))?;
        require(row.try_get::<i64>("", "people")? == 1
            && row.try_get::<i64>("", "memberships")? == 2
            && row.try_get::<i64>("", "users")? == 0,
            "One accountless Person must have two participations and no login")?;
        for (account_id, household_id) in [(71001,72001),(71002,72002)] {
            let tenant = access::begin(&fixture.runtime, &scope(account_id,household_id), None).await
                .map_err(|_| DbErr::Custom("Expected reader household scope".to_owned()))?;
            require(tenant.membership().role == "member", "Reader must be an ordinary household member")?;
            let found = visible(tenant.transaction(),73003).await?;
            let foreign = visible(tenant.transaction(),
                if household_id == 72001 { 73004 } else { 73002 }).await?;
            let foreign_linked = visible(tenant.transaction(),73101).await?;
            let foreign_accountless = visible(tenant.transaction(),73102).await?;
            let record = access::require_person_access(&tenant,73003,access::PersonAccess::Record).await;
            let view = access::require_person_access(&tenant,73003,access::PersonAccess::View).await;
            tenant.rollback().await
                .map_err(|_| DbErr::Custom("Unable to close reader scope".to_owned()))?;
            require(found, "Current household member must see the same dependent Person")?;
            require(!foreign && !foreign_linked && !foreign_accountless,
                "Foreign account-linked and accountless Persons must remain hidden")?;
            record.map_err(|_| DbErr::Custom("Member must be able to record care without a Person grant".to_owned()))?;
            view.map_err(|_| DbErr::Custom("Member must be able to view care without a Person grant".to_owned()))?;
        }
        require(matches!(access::begin(&fixture.runtime, &scope(71001,72003), None).await,
            Err(OperationError::Forbidden)),
            "Current membership in A must not permit selecting a foreign household")?;
        let target = access::begin(&fixture.runtime, &scope(71002,72002), None).await
            .map_err(|_| DbErr::Custom("B reader must have a live scope before subject departure".to_owned()))?;
        fixture.admin.execute_unprepared(
            "UPDATE public.household_memberships SET status='revoked',revoked_at=now(),
             permissions_version=permissions_version+1 WHERE id=74502"
        ).await?;
        let target_visible = visible(target.transaction(),73003).await?;
        let access = access::require_person_access(&target,73003,access::PersonAccess::View).await;
        target.rollback().await.map_err(|_| DbErr::Custom("Unable to close B scope".to_owned()))?;
        require(!target_visible && matches!(access,Err(OperationError::Forbidden)),
            "Dependent departure must remove authority even from an already-open B scope")?;
        let fresh_target = access::begin(&fixture.runtime, &scope(71002,72002), None).await
            .map_err(|_| DbErr::Custom("Subject departure must not revoke the B reader".to_owned()))?;
        let fresh_visible = visible(fresh_target.transaction(),73003).await?;
        fresh_target.rollback().await.map_err(|_| DbErr::Custom("Unable to close fresh B scope".to_owned()))?;
        require(!fresh_visible, "Fresh B scopes must also exclude the departed subject")?;
        let origin = access::begin(&fixture.runtime, &scope(71001,72001), None).await
            .map_err(|_| DbErr::Custom("A reader remains a current member".to_owned()))?;
        let origin_visible = visible(origin.transaction(),73003).await?;
        origin.rollback().await.map_err(|_| DbErr::Custom("Unable to close A scope".to_owned()))?;
        require(origin_visible, "Dependent departure from B must preserve A participation")?;
        Ok(())
    }.await;
    fixture.close().await;
    result
}
