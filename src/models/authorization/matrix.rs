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
fn guardian_recovery_requires_current_same_household_manager_and_minor() {
    for (role, expected) in [("owner", true), ("administrator", true), ("member", false)] {
        assert_eq!(
            evaluate(
                member(role),
                subject(),
                "recover_minor_guardian",
                json!({"person_type": 1, "has_capacity": false, "participating": true}),
            ),
            expected,
        );
    }
    for facts in [
        json!({"person_type": 0, "has_capacity": false, "participating": true}),
        json!({"person_type": 2, "has_capacity": false, "participating": true}),
        json!({"person_type": 1, "has_capacity": true, "participating": true}),
        json!({"person_type": 1, "has_capacity": false, "participating": false}),
    ] {
        assert!(!evaluate(
            member("owner"),
            subject(),
            "recover_minor_guardian",
            facts
        ));
    }
}

#[test]
fn person_participation_self_and_legacy_manage_preserve_separate_authorities() {
    for role in ["owner", "administrator", "member"] {
        for (action, facts) in [
            ("view_person", json!({"participating":true})),
            ("record_person", json!({"participating":true})),
            ("manage_self", json!({"own":true})),
            ("manage_person", grant("manage")),
        ] {
            assert!(evaluate(member(role), subject(), action, facts.clone()));
            assert!(!evaluate(member(role), subject(), action, json!({})));
            let mut foreign = subject();
            foreign["household_id"] = json!(22);
            assert!(!evaluate(member(role), foreign, action, facts.clone()));
            for (field, value) in [
                ("household_id", json!(22)),
                ("status", json!("inactive")),
                ("revoked", json!(true)),
            ] {
                let mut principal = member(role);
                principal[field] = value;
                assert!(!evaluate(principal, subject(), action, facts.clone()));
            }
        }
        for (action, facts) in [
            ("view_person", json!({"participating":false})),
            ("record_person", json!({"participating":false})),
            ("manage_self", json!({"own":false})),
        ] {
            assert!(!evaluate(member(role), subject(), action, facts));
        }
        for level in ["view", "record", "unknown"] {
            assert!(!evaluate(
                member(role),
                subject(),
                "manage_person",
                grant(level)
            ));
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
        assert!(!evaluate(
            member("member"),
            subject(),
            "manage_person",
            facts
        ));
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
        "manage_person",
        grant("manage")
    ));
}

#[test]
fn new_person_delegation_stays_closed_while_owner_change_rules_remain() {
    for role in ["member", "administrator", "owner"] {
        for kind in [0, 1, 2] {
            for capacity in [true, false] {
                assert!(
                    !evaluate(
                        member(role),
                        subject(),
                        "delegate_person",
                        json!({"person_type":kind,"has_capacity":capacity})
                    ),
                    "{role}/{kind}/{capacity}"
                );
            }
        }
    }
    for role in ["member", "administrator", "owner"] {
        for next_owner in [true, false] {
            assert_eq!(
                evaluate(
                    member(role),
                    subject(),
                    "change_owner",
                    json!({"next_owner":next_owner})
                ),
                role == "owner" && !next_owner
            );
        }
    }
}

#[test]
fn support_policy_requires_trusted_live_context_and_never_fills_missing_facts() {
    let account = json!({"id":71,"active":true,"platform_admin":true});
    let facts = json!({"trusted":true,"household_id":21,"ended":false,"expired":false,"expires_at":101,"now":100});
    assert!(account_decision(
        account.clone(),
        "Resource",
        subject(),
        "support_household",
        facts.clone()
    ));
    assert!(!account_decision(
        account.clone(),
        "Resource",
        subject(),
        "support_household",
        json!({})
    ));
    for (field, value) in [
        ("trusted", json!(false)),
        ("household_id", json!(22)),
        ("ended", json!(true)),
        ("expired", json!(true)),
        ("expires_at", json!(100)),
    ] {
        let mut changed = facts.clone();
        changed[field] = value;
        assert!(!account_decision(
            account.clone(),
            "Resource",
            subject(),
            "support_household",
            changed
        ));
    }
    for (field, value) in [("active", json!(false)), ("platform_admin", json!(false))] {
        let mut changed = account.clone();
        changed[field] = value;
        assert!(!account_decision(
            changed,
            "Resource",
            subject(),
            "support_household",
            facts.clone()
        ));
    }
}

#[test]
fn support_approval_and_end_respect_owner_and_admin_boundaries() {
    let owner_account = json!({"id":71,"active":true,"platform_admin":false});
    let admin_account = json!({"id":72,"active":true,"platform_admin":true});
    for (owner, expected) in [(true, true), (false, false)] {
        assert_eq!(
            account_decision(
                owner_account.clone(),
                "Resource",
                subject(),
                "support_approve",
                json!({"household_id":21,"owner":owner})
            ),
            expected
        );
    }
    assert!(!account_decision(
        admin_account.clone(),
        "Resource",
        subject(),
        "support_approve",
        json!({"household_id":21,"owner":false})
    ));
    assert!(account_decision(
        admin_account.clone(),
        "Resource",
        subject(),
        "support_end",
        json!({"household_id":21,"owner":false})
    ));
    assert!(account_decision(
        owner_account.clone(),
        "Resource",
        subject(),
        "support_end",
        json!({"household_id":21,"owner":true})
    ));
    let mut foreign = subject();
    foreign["household_id"] = json!(22);
    assert!(!account_decision(
        admin_account.clone(),
        "Resource",
        foreign,
        "support_end",
        json!({"household_id":21,"owner":false})
    ));
    assert!(!account_decision(
        json!({"id":71,"active":false,"platform_admin":false}),
        "Resource",
        subject(),
        "support_end",
        json!({"household_id":21,"owner":true})
    ));
}

#[test]
fn platform_user_listing_requires_active_account_and_active_administrator() {
    let account = json!({"id":71,"active":true,"platform_admin":true});
    for action in [
        "read_platform_users",
        "write_platform_users",
        "read_platform_settings",
        "write_platform_settings",
        "platform_owner_recovery",
        "support_request",
        "support_activate",
    ] {
        assert!(evaluate_account(account.clone(), action), "{action}");
        for (field, value) in [("active", json!(false)), ("platform_admin", json!(false))] {
            let mut changed = account.clone();
            changed[field] = value;
            assert!(!evaluate_account(changed, action), "{action} {field}");
        }
        assert!(!evaluate_account(member("owner"), action), "{action}");
        assert!(!evaluate_account(json!({}), action), "{action}");
    }
    assert!(!evaluate_account(account, "manage_household"));
}

#[test]
fn malformed_or_unknown_requests_fail_closed() {
    assert!(!evaluate(member("owner"), subject(), "unknown", json!({})));
    let mut missing = member("owner");
    missing.as_object_mut().unwrap().remove("status");
    assert!(!evaluate(missing, subject(), "manage_household", json!({})));
    let mut invalid = grant("manage");
    assert!(evaluate(
        member("owner"),
        subject(),
        "manage_person",
        invalid.clone()
    ));
    invalid["now"] = json!("100");
    assert!(!evaluate(
        member("owner"),
        subject(),
        "manage_person",
        invalid
    ));
}
