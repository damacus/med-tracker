use super::*;

fn member(role: &str) -> Value {
    json!({"id":11,"household_id":21,"role":role,"status":"active","revoked":false})
}

fn subject() -> Value {
    json!({"id":31,"household_id":21})
}

fn grant(level: &str) -> Value {
    json!({"household_id":21,"membership_id":11,"person_id":31,"level":level,"revoked":false,"expiring":false,"expires_at":0,"now":100})
}

#[test]
fn embedded_schema_and_policies_validate_strictly() {
    let (schema, _) =
        Schema::from_cedarschema_str(include_str!("schema.cedarschema")).expect("embedded schema");
    let policies = include_str!("policies.cedar")
        .parse::<PolicySet>()
        .expect("embedded policies");
    let validation = Validator::new(schema).validate(&policies, ValidationMode::Strict);
    assert!(
        validation.validation_passed(),
        "{:?}",
        validation.validation_errors().collect::<Vec<_>>()
    );
    assert!(engine().is_some());
}

#[test]
fn household_management_requires_current_same_household_manager() {
    for (role, allowed) in [
        ("owner", true),
        ("administrator", true),
        ("member", false),
        ("admin", false),
        ("", false),
    ] {
        assert_eq!(
            evaluate(member(role), subject(), "manage_household", json!({})),
            allowed,
            "role {role}"
        );
    }
    for (field, value) in [
        ("household_id", json!(22)),
        ("status", json!("inactive")),
        ("revoked", json!(true)),
    ] {
        let mut facts = member("owner");
        facts[field] = value;
        assert!(!evaluate(facts, subject(), "manage_household", json!({})));
    }
}

#[test]
fn person_permissions_follow_the_grant_hierarchy_without_role_bypass() {
    for role in ["owner", "administrator", "member"] {
        for (level, expected) in [
            ("view", [true, false, false]),
            ("record", [true, true, false]),
            ("manage", [true, true, true]),
            ("unknown", [false, false, false]),
        ] {
            for (action, allowed) in ["view_person", "record_person", "manage_person"]
                .into_iter()
                .zip(expected)
            {
                assert_eq!(
                    evaluate(member(role), subject(), action, grant(level)),
                    allowed,
                    "{role}/{level}/{action}"
                );
            }
        }
    }
}

#[test]
fn grants_are_bound_to_household_membership_person_and_live_expiry() {
    for (field, value) in [
        ("household_id", json!(22)),
        ("membership_id", json!(12)),
        ("person_id", json!(32)),
        ("revoked", json!(true)),
    ] {
        let mut facts = grant("manage");
        facts[field] = value;
        assert!(!evaluate(member("member"), subject(), "view_person", facts));
    }
    for (deadline, expected) in [(99, false), (100, false), (101, true)] {
        let mut facts = grant("manage");
        facts["expiring"] = json!(true);
        facts["expires_at"] = json!(deadline);
        assert_eq!(
            evaluate(member("member"), subject(), "manage_person", facts),
            expected
        );
    }
    let mut foreign = subject();
    foreign["household_id"] = json!(22);
    assert!(!evaluate(
        member("owner"),
        foreign,
        "view_person",
        grant("manage")
    ));
}

#[test]
fn delegated_person_management_preserves_capacity_and_owner_rules() {
    for role in ["member", "administrator", "owner"] {
        for kind in [0, 1, 2] {
            for capacity in [true, false] {
                let expected = role != "member" || (kind != 0 && !capacity);
                assert_eq!(
                    evaluate(
                        member(role),
                        subject(),
                        "delegate_person",
                        json!({"person_type":kind,"has_capacity":capacity})
                    ),
                    expected
                );
            }
        }
    }
    for role in ["member", "administrator", "owner"] {
        for platform in [true, false] {
            for next_owner in [true, false] {
                assert_eq!(
                    evaluate(
                        member(role),
                        subject(),
                        "change_owner",
                        json!({"platform_admin":platform,"next_owner":next_owner})
                    ),
                    platform || (role == "owner" && !next_owner)
                );
            }
        }
    }
}

#[test]
fn support_policy_requires_trusted_live_context_and_never_fills_missing_facts() {
    let facts = json!({"trusted":true,"platform_admin":true,"household_id":21,"ended":false,"expired":false,"expires_at":101,"now":100});
    assert!(evaluate(
        member("member"),
        subject(),
        "support_household",
        facts.clone()
    ));
    assert!(!evaluate(
        member("owner"),
        subject(),
        "support_household",
        json!({})
    ));
    for (field, value) in [
        ("trusted", json!(false)),
        ("platform_admin", json!(false)),
        ("household_id", json!(22)),
        ("ended", json!(true)),
        ("expired", json!(true)),
        ("expires_at", json!(100)),
    ] {
        let mut changed = facts.clone();
        changed[field] = value;
        assert!(!evaluate(
            member("owner"),
            subject(),
            "support_household",
            changed
        ));
    }
}

#[test]
fn malformed_or_unknown_requests_fail_closed() {
    assert!(!evaluate(member("owner"), subject(), "unknown", json!({})));
    let mut missing = member("owner");
    missing.as_object_mut().unwrap().remove("status");
    assert!(!evaluate(missing, subject(), "manage_household", json!({})));
    let mut invalid = grant("manage");
    invalid["now"] = json!("100");
    assert!(!evaluate(
        member("owner"),
        subject(),
        "view_person",
        invalid
    ));
}
