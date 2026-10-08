use super::*;

pub(super) struct RequestAudit {
    method: &'static str,
    controller: &'static str,
    action: &'static str,
    policy: &'static str,
    query: &'static str,
}

impl RequestAudit {
    pub fn report(is_reviews: bool) -> Self {
        Self {
            method: "GET",
            controller: if is_reviews {
                "api/v1/medication_review_reports"
            } else {
                "api/v1/health_history_reports"
            },
            action: "show",
            policy: if is_reviews {
                "MedicationReviewPromptPolicy"
            } else {
                "ReportPolicy"
            },
            query: "index?",
        }
    }
    pub fn sync(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: if action == "create" {
                "api/v1/sync/batches"
            } else {
                "api/v1/sync/feeds"
            },
            action,
            policy: "HouseholdPolicy",
            query: match action {
                "snapshot" => "snapshot?",
                "changes" => "changes?",
                _ => "create?",
            },
        }
    }
    pub fn dose_occurrence(
        method: &'static str,
        action: &'static str,
        source: &'static str,
    ) -> Self {
        Self {
            method,
            controller: "api/v1/dose_occurrences",
            action,
            policy: if source == "schedule" {
                "SchedulePolicy"
            } else {
                "PersonMedicationPolicy"
            },
            query: match action {
                "take" => "take?",
                "not_taken" => "not_taken?",
                "reopen" => "reopen?",
                _ => "index?",
            },
        }
    }
    pub fn pause_period(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/medication_pause_periods",
            action,
            policy: "MedicationPausePeriodPolicy",
            query: match action {
                "create" => "create?",
                "resume" => "resume?",
                _ => "index?",
            },
        }
    }
    pub fn assignment(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/person_medications",
            action,
            policy: "PersonMedicationPolicy",
            query: match action {
                "create" => "create?",
                "update" => "update?",
                "show" => "show?",
                "pause" => "pause?",
                "resume" => "resume?",
                _ => "index?",
            },
        }
    }
    pub fn push(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/push_subscriptions",
            action,
            policy: "PushSubscriptionPolicy",
            query: if action == "create" {
                "create?"
            } else {
                "destroy?"
            },
        }
    }
    pub fn treatment(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/schedules",
            action,
            policy: "SchedulePolicy",
            query: match action {
                "create" => "create?",
                "update" => "update?",
                "show" => "show?",
                "pause" => "pause?",
                "resume" => "resume?",
                _ => "index?",
            },
        }
    }
    pub fn dosage(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/dosage_options",
            action,
            policy: "DosageOptionPolicy",
            query: match action {
                "create" => "create?",
                "update" => "update?",
                "show" => "show?",
                _ => "index?",
            },
        }
    }
    pub fn invitations(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/admin/invitations",
            action,
            policy: "HouseholdInvitationPolicy",
            query: match action {
                "create" => "create?",
                "destroy" => "destroy?",
                "resend" => "resend?",
                _ => "index?",
            },
        }
    }
    pub fn administration(
        method: &'static str,
        resource: &'static str,
        action: &'static str,
    ) -> Self {
        let (controller, policy) = match resource {
            "settings" => ("api/v1/admin/settings", "HouseholdPolicy"),
            "memberships" => ("api/v1/admin/memberships", "HouseholdMembershipPolicy"),
            _ => (
                "api/v1/admin/person_access_grants",
                "PersonAccessGrantPolicy",
            ),
        };
        Self {
            method,
            controller,
            action,
            policy,
            query: match action {
                "show" => "show?",
                "create" => "create?",
                "update" => "update?",
                "destroy" => "destroy?",
                _ => "index?",
            },
        }
    }
    pub fn person(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/people",
            action,
            policy: "PersonPolicy",
            query: match action {
                "create" => "create?",
                "update" => "update?",
                "show" => "show?",
                _ => "index?",
            },
        }
    }
    pub fn location_create() -> Self {
        Self::location("POST", "create")
    }
    pub fn location(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/locations",
            action,
            policy: "LocationPolicy",
            query: match action {
                "create" => "create?",
                "update" => "update?",
                "destroy" => "destroy?",
                "show" => "show?",
                _ => "index?",
            },
        }
    }
    pub fn medication(method: &'static str, action: &'static str) -> Self {
        Self {
            method,
            controller: "api/v1/medications",
            action,
            policy: "MedicationPolicy",
            query: match action {
                "create" => "create?",
                "index" => "index?",
                "show" => "show?",
                "mark_as_ordered" => "mark_as_ordered?",
                "mark_as_received" => "mark_as_received?",
                _ => "update?",
            },
        }
    }
    pub fn removal(history: bool) -> Self {
        Self {
            method: if history { "GET" } else { "POST" },
            controller: "api/v1/stock_removals",
            action: if history { "index" } else { "create" },
            policy: "MedicationPolicy",
            query: "update?",
        }
    }
    pub fn take() -> Self {
        Self {
            method: "POST",
            controller: "api/v1/medication_takes",
            action: "create",
            policy: "MedicationTakePolicy",
            query: "create?",
        }
    }
    pub fn take_history() -> Self {
        Self {
            method: "GET",
            controller: "api/v1/medication_takes",
            action: "index",
            policy: "MedicationTakePolicy",
            query: "index?",
        }
    }
    pub fn stock() -> Self {
        Self {
            method: "PATCH",
            controller: "api/v1/medications",
            action: "adjust_inventory",
            policy: "MedicationPolicy",
            query: "adjust_inventory?",
        }
    }
}

pub(super) async fn record(
    tenant: &TenantTransaction,
    provenance: &CredentialProvenance,
    request: RequestAudit,
    status: StatusCode,
) -> std::result::Result<(), OperationError> {
    let (method, prefix) = match provenance.method {
        CredentialMethod::ApiSession => ("api_session", "api_session"),
        CredentialMethod::ApiAppToken => ("api_app_token", "api_app_token"),
        CredentialMethod::OauthGrant => ("oauth", "oauth_grant"),
        CredentialMethod::PersonalApiKey => ("personal_api_key", "personal_api_key"),
        CredentialMethod::BrowserSession => ("browser_session", "browser_session"),
    };
    let mut context = json!({
        "actor_account_id": tenant.scope().actor.account_id, "actor_user_id": tenant.user_id(),
        "actor_membership_id": tenant.membership().id, "active_role": tenant.membership().role,
        "permissions_version": tenant.membership().permissions_version, "household_id": tenant.scope().household_id,
        "authentication_method": method, "session_reference": format!("{prefix}:{}", provenance.reference),
        "request_id": tenant.scope().request_id,
    });
    let success = status.is_success() || status == StatusCode::NOT_MODIFIED;
    if success {
        context["policy_class"] = json!(request.policy);
        context["policy_query"] = json!(request.query);
    }
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(tenant.scope().household_id), actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(tenant.membership().id)), event_type: Set("api.request".into()),
        request_id: Set(Some(tenant.scope().request_id.clone())), metadata: Set(json!({"http_method":request.method,"controller":request.controller,"action":request.action,"outcome":if success{"success"}else{"failure"},"status":status.as_u16()})),
        audit_context: Set(context), created_at: Set(now), updated_at: Set(now), ..Default::default()
    }.insert(tenant.transaction()).await.map_err(|_| OperationError::Unavailable)?;
    Ok(())
}
