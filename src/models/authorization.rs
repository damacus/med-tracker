use cedar_policy::{
    Authorizer, Context, Decision, Entities, EntityUid, PolicySet, Request, Response, Schema,
    ValidationMode, Validator,
};
use serde_json::{Value, json};
use std::sync::OnceLock;

use super::{
    access::PersonAccess,
    entities::{grant, membership, person},
};

#[cfg(test)]
mod matrix;

struct Engine {
    policies: PolicySet,
    schema: Schema,
}

fn engine() -> Option<&'static Engine> {
    static ENGINE: OnceLock<Option<Engine>> = OnceLock::new();
    ENGINE
        .get_or_init(|| {
            let (schema, _) =
                Schema::from_cedarschema_str(include_str!("authorization/schema.cedarschema"))
                    .ok()?;
            let policies = include_str!("authorization/policies.cedar")
                .parse::<PolicySet>()
                .ok()?;
            if !Validator::new(schema.clone())
                .validate(&policies, ValidationMode::Strict)
                .validation_passed()
            {
                return None;
            }
            Some(Engine { policies, schema })
        })
        .as_ref()
}

fn strict_decision(response: &Response) -> Decision {
    if response.diagnostics().errors().next().is_some() {
        Decision::Deny
    } else {
        response.decision()
    }
}

pub(crate) fn ready() -> bool {
    engine().is_some()
}

fn evaluate(member: Value, resource: Value, action: &str, facts: Value) -> bool {
    let Some(engine) = engine() else {
        return false;
    };
    let result = (|| {
        let principal: EntityUid = "Membership::\"actor\"".parse().ok()?;
        let resource_id: EntityUid = "Resource::\"target\"".parse().ok()?;
        let action: EntityUid = format!("Action::\"{action}\"").parse().ok()?;
        let entities = Entities::from_json_value(
            json!([
                {"uid":{"type":"Membership","id":"actor"},"attrs":member,"parents":[]},
                {"uid":{"type":"Resource","id":"target"},"attrs":resource,"parents":[]}
            ]),
            Some(&engine.schema),
        )
        .ok()?;
        let context = Context::from_json_value(facts, Some((&engine.schema, &action))).ok()?;
        let request = Request::new(
            principal,
            action,
            resource_id,
            context,
            Some(&engine.schema),
        )
        .ok()?;
        Some(
            strict_decision(&Authorizer::new().is_authorized(
                &request,
                &engine.policies,
                &entities,
            )) == Decision::Allow,
        )
    })();
    result.unwrap_or(false)
}

fn member_facts(member: &membership::Model) -> Value {
    json!({"id":member.id,"household_id":member.household_id,"role":member.role,"status":member.status,"revoked":member.revoked_at.is_some()})
}

pub(crate) fn household_manager(member: &membership::Model, household_id: i64) -> bool {
    evaluate(
        member_facts(member),
        json!({"id":household_id,"household_id":household_id}),
        "manage_household",
        json!({}),
    )
}

pub(crate) fn person_access(
    member: &membership::Model,
    subject: &person::Model,
    grant: &grant::Model,
    access: PersonAccess,
    now: chrono::NaiveDateTime,
) -> bool {
    let action = match access {
        PersonAccess::View => "view_person",
        PersonAccess::Record => "record_person",
        PersonAccess::Manage => "manage_person",
    };
    evaluate(
        member_facts(member),
        json!({"id":subject.id,"household_id":subject.household_id}),
        action,
        json!({
            "household_id":grant.household_id,"membership_id":grant.household_membership_id,
            "person_id":grant.person_id,"level":grant.access_level,"revoked":grant.revoked_at.is_some(),
            "expiring":grant.expires_at.is_some(),"expires_at":grant.expires_at.map_or(0,|time|time.and_utc().timestamp_micros()),
            "now":now.and_utc().timestamp_micros()
        }),
    )
}

pub(crate) fn may_delegate(member: &membership::Model, subject: &person::Model) -> bool {
    evaluate(
        member_facts(member),
        json!({"id":subject.id,"household_id":subject.household_id}),
        "delegate_person",
        json!({"person_type":subject.person_type,"has_capacity":subject.has_capacity}),
    )
}

pub(crate) fn may_change_owner(
    member: &membership::Model,
    household_id: i64,
    platform_admin: bool,
    next_owner: bool,
) -> bool {
    evaluate(
        member_facts(member),
        json!({"id":household_id,"household_id":household_id}),
        "change_owner",
        json!({"platform_admin":platform_admin,"next_owner":next_owner}),
    )
}

#[cfg(test)]
mod tests {
    use cedar_policy::{Authorizer, Context, Decision, Entities, PolicySet, Request};

    #[test]
    fn authorization_diagnostics_fail_closed_even_with_an_allow() {
        let policies: PolicySet = r#"
permit(principal, action, resource);
forbid(principal, action, resource) when { principal.missing };
"#
        .parse()
        .expect("test policy parses");
        let request = Request::new(
            "Account::\"1\"".parse().expect("principal"),
            "Action::\"view\"".parse().expect("action"),
            "Person::\"2\"".parse().expect("resource"),
            Context::empty(),
            None,
        )
        .expect("request");
        let response = Authorizer::new().is_authorized(&request, &policies, &Entities::empty());
        assert!(response.diagnostics().errors().next().is_some());
        assert_eq!(response.decision(), Decision::Allow);
        assert_eq!(super::strict_decision(&response), Decision::Deny);
    }
}
