use super::*;

async fn visible(db: &impl ConnectionTrait, person_id: i64) -> bool {
    people::Entity::find_by_id(person_id)
        .one(db)
        .await
        .unwrap()
        .is_some()
}

async fn account_count(db: &impl ConnectionTrait, person_id: i64) -> u64 {
    accounts::Entity::find()
        .filter(accounts::Column::PersonId.eq(person_id))
        .count(db)
        .await
        .unwrap()
}

#[tokio::test]
async fn accountless_person_participates_in_two_households_without_credentials() {
    let fixture = Fixture::new().await;
    set_role(&fixture, "member").await;
    let third = Scenario::new().await;
    set_role(&third, "owner").await;
    let foreign_dependent = person(&fixture.admin, "Synthetic foreign dependent", 2, false).await;
    membership(
        &fixture.admin,
        third.household.id,
        foreign_dependent.id,
        "member",
    )
    .await;
    let origin_only = person(&fixture.admin, "Synthetic origin dependent", 1, false).await;
    membership(
        &fixture.admin,
        fixture.household.id,
        origin_only.id,
        "member",
    )
    .await;
    let accounts_before = account_count(&fixture.admin, fixture.accountless.id).await;
    membership(
        &fixture.admin,
        fixture.household.id,
        fixture.accountless.id,
        "member",
    )
    .await;
    let target_membership = membership(
        &fixture.admin,
        fixture.other.household.id,
        fixture.accountless.id,
        "member",
    )
    .await;
    assert_eq!(
        account_count(&fixture.admin, fixture.accountless.id).await,
        accounts_before,
        "Dependent participation must not create an Account"
    );
    assert_eq!(accounts_before, 0);
    assert_eq!(
        people::Entity::find()
            .filter(people::Column::Id.eq(fixture.accountless.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        household_memberships::Entity::find()
            .filter(household_memberships::Column::PersonId.eq(fixture.accountless.id))
            .filter(household_memberships::Column::Status.eq("active"))
            .filter(household_memberships::Column::RevokedAt.is_null())
            .count(&fixture.admin)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        accounts::Entity::find()
            .filter(accounts::Column::PersonId.eq(fixture.accountless.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0,
        "One accountless Person must have two participations and no login"
    );
    for (reader_scope, foreign_person) in [
        (fixture.scope(), fixture.other.account.person_id),
        (fixture.other.scope(), origin_only.id),
    ] {
        let tenant = access::begin(&fixture.runtime, &reader_scope, None)
            .await
            .unwrap();
        assert_eq!(
            tenant.membership().role,
            "member",
            "Reader must be an ordinary household member"
        );
        let found = visible(tenant.transaction(), fixture.accountless.id).await;
        let foreign = visible(tenant.transaction(), foreign_person).await;
        let foreign_linked = visible(tenant.transaction(), third.account.person_id).await;
        let foreign_accountless = visible(tenant.transaction(), foreign_dependent.id).await;
        let record =
            access::require_person_access(&tenant, fixture.accountless.id, PersonAccess::Record)
                .await;
        let view =
            access::require_person_access(&tenant, fixture.accountless.id, PersonAccess::View)
                .await;
        tenant.rollback().await.unwrap();
        assert!(
            found,
            "Current household member must see the same dependent Person"
        );
        assert!(
            !foreign && !foreign_linked && !foreign_accountless,
            "Foreign account-linked and accountless Persons must remain hidden"
        );
        assert_eq!(
            record,
            Ok(()),
            "Member must be able to record care without a Person grant"
        );
        assert_eq!(
            view,
            Ok(()),
            "Member must be able to view care without a Person grant"
        );
    }
    assert!(
        matches!(
            access::begin(
                &fixture.runtime,
                &scope(fixture.account.id, third.household.id),
                None
            )
            .await,
            Err(OperationError::Forbidden)
        ),
        "Current membership in A must not permit selecting a foreign household"
    );
    let target = access::begin(&fixture.runtime, &fixture.other.scope(), None)
        .await
        .unwrap();
    let permissions_version = target_membership.permissions_version;
    let mut departed = target_membership.into_active_model();
    departed.status = Set("revoked".into());
    departed.revoked_at = Set(Some(now()));
    departed.permissions_version = Set(permissions_version + 1);
    departed.update(&fixture.admin).await.unwrap();
    let target_visible = visible(target.transaction(), fixture.accountless.id).await;
    let access =
        access::require_person_access(&target, fixture.accountless.id, PersonAccess::View).await;
    target.rollback().await.unwrap();
    assert!(
        !target_visible && matches!(access, Err(OperationError::Forbidden)),
        "Dependent departure must remove authority even from an already-open B scope"
    );
    let fresh_target = access::begin(&fixture.runtime, &fixture.other.scope(), None)
        .await
        .unwrap();
    let fresh_visible = visible(fresh_target.transaction(), fixture.accountless.id).await;
    fresh_target.rollback().await.unwrap();
    assert!(
        !fresh_visible,
        "Fresh B scopes must also exclude the departed subject"
    );
    let origin = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    let origin_visible = visible(origin.transaction(), fixture.accountless.id).await;
    origin.rollback().await.unwrap();
    assert!(
        origin_visible,
        "Dependent departure from B must preserve A participation"
    );
    third.close().await;
    fixture.close().await;
}
