use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::redirect::Policy;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;
use url::{Host, Url};

#[derive(Deserialize)]
pub struct Fixture {
    pub push_api_household_id: i64,
    pub push_api_access_token: String,
    pub push_web_household_id: i64,
    pub push_web_household_slug: String,
    pub push_web_email: String,
    pub push_observer_household_id: i64,
    pub push_observer_access_token: String,
    pub push_fatal_household_id: i64,
    pub push_fatal_household_slug: String,
    pub push_fatal_email: String,
    pub push_fatal_access_token: String,
    pub web_device_household_id: i64,
    pub web_device_household_slug: String,
    pub web_device_email: String,
    pub web_device_access_token: String,
    pub retained_household_slug: String,
    pub retained_household_id: i64,
    pub retained_email: String,
    pub retained_schedule_id: i64,
    pub retained_medication_id: i64,
    pub retained_foreign_household_slug: String,
    pub offline_csrf_household_slug: String,
    pub offline_csrf_household_id: i64,
    pub offline_csrf_email: String,
    pub offline_csrf_schedule_id: i64,
    pub offline_csrf_medication_id: i64,
    pub offline_future_household_slug: String,
    pub offline_future_household_id: i64,
    pub offline_future_email: String,
    pub offline_future_schedule_id: i64,
    pub offline_future_medication_id: i64,
    pub offline_eligibility_household_slug: String,
    pub offline_eligibility_email: String,
    pub offline_inactive_schedule_id: i64,
    pub offline_cooldown_schedule_id: i64,
    pub offline_expired_schedule_id: i64,
    pub sync_action_household_id: i64,
    pub sync_action_access_token: String,
    pub sync_action_view_access_token: String,
    pub sync_action_source_schedule_portable_id: String,
    pub sync_action_source_assignment_portable_id: String,
    pub sync_action_reorder_first_portable_id: String,
    pub sync_action_reorder_second_portable_id: String,
    pub sync_period_household_id: i64,
    pub sync_period_membership_id: i64,
    pub sync_period_access_token: String,
    pub sync_period_view_access_token: String,
    pub sync_period_schedule_portable_id: String,
    pub sync_period_assignment_portable_id: String,
    pub platform_admin_email: String,
    pub platform_admin_account_id: i64,
    pub platform_target_email: String,
    pub platform_target_user_id: i64,
    pub platform_promote_membership_id: i64,
    pub platform_promote_email: String,
    pub platform_denied_user_id: i64,
    pub platform_denied_membership_id: i64,
    pub platform_denied_email: String,
    pub platform_household_id: i64,
    pub platform_support_household_id: i64,
    pub platform_support_household_slug: String,
    pub platform_support_audit_token: String,
    pub platform_expired_support_session_id: i64,
    pub platform_unrelated_household_slug: String,
    pub web_household_slug: String,
    pub web_foreign_barcode: String,
    pub web_foreign_display: String,
    pub web_view_email: String,
    pub web_feed_email: String,
    pub web_managed_person_name: String,
    pub web_hidden_person_name: String,
    pub web_ai_paid_slug: String,
    pub web_ai_paid_email: String,
    pub web_ai_paid_member_email: String,
    pub web_ai_free_slug: String,
    pub web_ai_free_email: String,
    pub web_ai_ip_slug: String,
    pub web_ai_ip_email: String,
    pub web_ai_user_slug: String,
    pub web_ai_user_email: String,
    pub web_people_slug: String,
    pub web_people_email: String,
    pub web_people_delete_id: i64,
    pub web_people_view_target_id: i64,
    pub web_people_history_id: i64,
    pub web_people_history_schedule_id: i64,
    pub web_people_foreign_slug: String,
    pub web_people_foreign_email: String,
    pub web_people_foreign_id: i64,
    pub web_people_member_email: String,
    pub web_people_view_member_email: String,
    pub profile_household_id: i64,
    pub avatar_household_id: i64,
    pub web_avatar_household_slug: String,
    pub web_avatar_email: String,
    pub web_avatar_person_id: i64,
    pub web_avatar_hidden_person_id: i64,
    pub avatar_invalid_household_slug: String,
    pub avatar_access_token: String,
    pub avatar_invalid_household_id: i64,
    pub avatar_invalid_access_token: String,
    pub profile_account_id: i64,
    pub profile_email: String,
    pub profile_person_id: i64,
    pub profile_access_token: String,
    pub profile_view_account_id: i64,
    pub profile_view_person_id: i64,
    pub profile_view_access_token: String,
    pub profile_revoke_person_id: i64,
    pub profile_revoke_membership_id: i64,
    pub profile_revoke_grant_id: i64,
    pub profile_revoke_access_token: String,
    pub profile_revoke_mobile_token: String,
    pub profile_signed_blob_id: String,
    pub upload_blob_signed_id: String,
    pub upload_disk_write_token: String,
    pub upload_variation_key: String,
    pub lookup_paid_household_id: i64,
    pub lookup_paid_account_id: i64,
    pub lookup_paid_membership_id: i64,
    pub lookup_paid_access_token: String,
    pub lookup_barcode: String,
    pub lookup_hidden_barcode: String,
    pub lookup_code: String,
    pub lookup_display: String,
    pub portable_source_household_id: i64,
    pub portable_source_account_id: i64,
    pub portable_source_membership_id: i64,
    pub portable_source_access_token: String,
    pub portable_source_person_name: String,
    pub portable_source_person_portable_id: String,
    pub portable_source_location_portable_id: String,
    pub portable_source_medication_portable_id: String,
    pub portable_source_schedule_portable_id: String,
    pub portable_target_household_id: i64,
    pub portable_target_account_id: i64,
    pub portable_target_membership_id: i64,
    pub portable_target_access_token: String,
    pub portable_target_app_token: String,
    pub portable_target_mobile_token: String,
    pub portable_member_access_token: String,
    pub portable_revoked_access_token: String,
    pub portable_locked_access_token: String,
    pub portable_numeric_bundle: serde_json::Value,
    pub portable_conflict_bundle: serde_json::Value,
    pub portable_malformed_bundle: serde_json::Value,
    pub access_token: String,
    pub account_id: i64,
    pub primary_email: String,
    pub user_id: i64,
    pub household_id: i64,
    pub household_slug: String,
    pub household_name: String,
    pub owner_membership_id: i64,
    pub revoked_owner_app_token_id: i64,
    pub foreign_household_id: i64,
    pub foreign_household_slug: String,
    pub foreign_membership_id: i64,
    pub foreign_app_token_id: i64,
    pub foreign_app_token: String,
    pub foreign_access_token: String,
    pub fhir_patient_scope_token: String,
    pub fhir_revoked_scope_token: String,
    pub foreign_email: String,
    pub manager_membership_id: i64,
    pub manager_access_token: String,
    pub invitation_authority_membership_id: i64,
    pub invitation_authority_access_token: String,
    pub invitation_authority_id: i64,
    pub token_authority_membership_id: i64,
    pub token_authority_access_token: String,
    pub manager_app_token_id: i64,
    pub manager_app_token: String,
    pub invitation_accept_id: i64,
    pub invitation_accept_email: String,
    pub invitation_accept_account_id: i64,
    pub invitation_accept_access_token: String,
    pub invitation_accept_token: String,
    pub invitation_expired_id: i64,
    pub invitation_expired_email: String,
    pub invitation_expired_account_id: i64,
    pub invitation_expired_access_token: String,
    pub invitation_expired_token: String,
    pub invitation_revoked_id: i64,
    pub invitation_revoked_access_token: String,
    pub invitation_revoked_token: String,
    pub invitation_rotation_id: i64,
    pub invitation_rotation_email: String,
    pub invitation_rotation_access_token: String,
    pub invitation_rotation_token: String,
    pub invitation_mobile_token: String,
    pub invitation_mobile_oauth_token: String,
    pub invitation_duplicate_email: String,
    pub admin_target_membership_id: i64,
    pub admin_target_access_token: String,
    pub admin_owner_patch_membership_id: i64,
    pub admin_manager_put_membership_id: i64,
    pub admin_manager_put_access_token: String,
    pub admin_owner_put_membership_id: i64,
    pub admin_owner_put_access_token: String,
    pub admin_invalid_membership_id: i64,
    pub admin_revoke_membership_id: i64,
    pub admin_revoke_access_token: String,
    pub last_owner_household_id: i64,
    pub last_owner_membership_id: i64,
    pub last_owner_access_token: String,
    pub session_id: i64,
    pub revocable_session_id: i64,
    pub revocable_access_token: String,
    pub medication_read_revoked_access_token: String,
    pub medication_read_stale_access_token: String,
    pub logout_access_token: String,
    pub expired_access_token: String,
    pub locked_access_token: String,
    pub auth_deactivated_household_id: i64,
    pub auth_deactivated_access_token: String,
    pub auth_deactivated_app_token: String,
    pub auth_inactive_household_id: i64,
    pub auth_inactive_access_token: String,
    pub auth_operational_states: serde_json::Value,
    pub auth_suspended_household_id: i64,
    pub auth_suspended_access_token: String,
    pub auth_role_household_id: i64,
    pub auth_role_member_membership_id: i64,
    pub auth_role_owner_access_token: String,
    pub auth_role_member_access_token: String,
    pub auth_role_member_app_token: String,
    pub auth_role_member_oauth_token: String,
    pub oauth_client_id: String,
    pub oauth_redirect_uri: String,
    pub smart_client_id: String,
    pub smart_redirect_uri: String,
    pub smart_patient_portable_id: String,
    pub user_person_id: i64,
    pub managed_person_id: i64,
    pub managed_person_portable_id: String,
    pub hidden_person_id: i64,
    pub hidden_person_portable_id: String,
    pub foreign_person_id: i64,
    pub foreign_person_portable_id: String,
    pub foreign_person_name: String,
    pub view_access_token: String,
    pub feed_access_token: String,
    pub cursor_boundary_location_portable_id: String,
    pub view_account_id: i64,
    pub view_membership_id: i64,
    pub delegated_access_token: String,
    pub replay_access_token: String,
    pub replay_mobile_access_token: String,
    pub replay_grant_id: i64,
    pub view_owner_access_token: String,
    pub care_access_token: String,
    pub grant_target_membership_id: i64,
    pub primary_location_id: i64,
    pub primary_location_portable_id: String,
    pub hidden_location_portable_id: String,
    pub historical_location_portable_id: String,
    pub historical_medication_portable_id: String,
    pub historical_medication_id: i64,
    pub historical_medication_name: String,
    pub foreign_location_id: i64,
    pub foreign_location_portable_id: String,
    pub foreign_location_name: String,
    pub managed_medication_id: i64,
    pub forecast_medication_id: i64,
    pub managed_medication_portable_id: String,
    pub managed_medication_name: String,
    pub visible_low_stock_portable_id: String,
    pub hidden_low_stock_portable_id: String,
    pub foreign_low_stock_portable_id: String,
    pub managed_dosage_portable_id: String,
    pub hidden_medication_id: i64,
    pub medication_read_household_id: i64,
    pub medication_read_delegated_access_token: String,
    pub medication_read_secondary_access_token: String,
    pub medication_read_unlinked_id: i64,
    pub medication_read_hidden_id: i64,
    pub hidden_medication_portable_id: String,
    pub foreign_medication_id: i64,
    pub foreign_medication_portable_id: String,
    pub foreign_medication_name: String,
    pub managed_assignment_id: i64,
    pub managed_assignment_portable_id: String,
    pub fhir_stopped_assignment_portable_id: String,
    pub managed_assignment_updated_at: String,
    pub retired_assignment_portable_id: String,
    pub retired_assignment_period_id: String,
    pub hidden_assignment_id: i64,
    pub hidden_assignment_portable_id: String,
    pub foreign_assignment_id: i64,
    pub foreign_assignment_portable_id: String,
    pub hidden_pause_period_id: String,
    pub foreign_pause_period_id: String,
    pub managed_schedule_id: i64,
    pub managed_schedule_portable_id: String,
    pub fhir_stopped_schedule_portable_id: String,
    pub historical_schedule_portable_id: String,
    pub managed_occurrence_portable_id: String,
    pub managed_take_portable_id: String,
    pub hidden_take_portable_id: String,
    pub foreign_take_portable_id: String,
    pub managed_preference_portable_id: String,
    pub hidden_preference_portable_id: String,
    pub foreign_preference_portable_id: String,
    pub managed_health_event_portable_id: String,
    pub managed_health_event_id: i64,
    pub managed_health_event_title: String,
    pub managed_side_effect_title: String,
    pub earlier_health_event_id: i64,
    pub earlier_health_event_title: String,
    pub hidden_health_event_portable_id: String,
    pub foreign_health_event_portable_id: String,
    pub hidden_schedule_id: i64,
    pub hidden_schedule_portable_id: String,
    pub foreign_schedule_id: i64,
    pub foreign_schedule_portable_id: String,
    pub foreign_dosage_id: i64,
    pub foreign_dosage_portable_id: String,
    pub hidden_dosage_id: i64,
    pub hidden_dosage_portable_id: String,
    pub hidden_health_event_id: i64,
    pub foreign_health_event_id: i64,
    pub managed_review_prompt_id: i64,
    pub second_review_prompt_id: i64,
    pub low_signal_review_prompt_id: i64,
    pub edit_review_prompt_id: i64,
    pub invalid_review_prompt_id: i64,
    pub hidden_review_prompt_id: i64,
    pub foreign_review_prompt_id: i64,
}

pub struct Target {
    client: Client,
    origin: Url,
}

impl Target {
    pub fn from_env() -> Self {
        let raw = env::var("CONTRACT_BASE_URL").expect("CONTRACT_BASE_URL is required");
        let origin = Url::parse(&raw).expect("CONTRACT_BASE_URL must be a URL");
        assert!(
            matches!(origin.scheme(), "http" | "https"),
            "HTTP(S) target required"
        );
        assert!(
            origin.username().is_empty() && origin.password().is_none(),
            "URL credentials are forbidden"
        );
        assert!(
            origin.path() == "/" && origin.query().is_none() && origin.fragment().is_none(),
            "target must be an origin"
        );
        let local = match origin.host() {
            Some(Host::Domain("localhost")) => true,
            Some(Host::Ipv4(ip)) => ip.is_loopback(),
            Some(Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if !local {
            let approved = env::var("CONTRACT_APPROVED_ORIGIN").unwrap_or_default();
            assert_eq!(
                origin.as_str().trim_end_matches('/'),
                approved.trim_end_matches('/'),
                "non-local target requires exact CONTRACT_APPROVED_ORIGIN"
            );
            assert_eq!(
                origin.scheme(),
                "https",
                "approved remote targets must use HTTPS"
            );
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(Policy::none())
            .cookie_store(true)
            .no_proxy()
            .build()
            .expect("HTTP client");
        Self { client, origin }
    }

    pub fn get(&self, path: &str, token: Option<&str>) -> Response {
        self.authorize(self.client.get(self.url(path)), token)
            .send()
            .expect("target must respond")
    }

    pub fn get_with_header(
        &self,
        path: &str,
        token: &str,
        name: &'static str,
        value: &str,
    ) -> Response {
        self.authorize(self.client.get(self.url(path)), Some(token))
            .header(name, value)
            .send()
            .expect("target must respond")
    }

    pub fn get_html(&self, path: &str) -> Response {
        self.client
            .get(self.url(path))
            .header("Accept", "text/html")
            .send()
            .expect("target must respond")
    }

    pub fn get_html_with_header(&self, path: &str, name: &'static str, value: &str) -> Response {
        self.client
            .get(self.url(path))
            .header("Accept", "text/html")
            .header(name, value)
            .send()
            .expect("target must respond")
    }

    pub fn get_html_from_local_client(&self, path: &str, client_ip: &str) -> Response {
        self.require_local_write();
        self.client
            .get(self.url(path))
            .header("Accept", "text/html")
            .header("X-Forwarded-For", client_ip)
            .send()
            .expect("target must respond")
    }

    pub fn get_from_local_client(&self, path: &str, client_ip: &str) -> Response {
        self.require_local_write();
        self.client
            .get(self.url(path))
            .header("Accept", "application/json")
            .header("X-Forwarded-For", client_ip)
            .send()
            .expect("target must respond")
    }

    pub fn delete(&self, path: &str, token: Option<&str>) -> Response {
        self.require_local_write();
        self.authorize(self.client.delete(self.url(path)), token)
            .send()
            .expect("target must respond")
    }

    pub fn delete_json(&self, path: &str) -> Response {
        self.require_local_write();
        self.client
            .delete(self.url(path))
            .header("Accept", "application/json")
            .send()
            .expect("target must respond")
    }

    pub fn delete_web_json(&self, path: &str, csrf: &str) -> Response {
        self.require_local_write();
        self.client
            .delete(self.url(path))
            .header("Accept", "application/json")
            .header("X-CSRF-Token", csrf)
            .send()
            .expect("target must respond")
    }

    pub fn post_form(&self, path: &str, fields: &[(&str, &str)]) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "application/json")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn post_html_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "text/html")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn patch_html_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .patch(self.url(path))
            .header("Accept", "text/html")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn put_html_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .put(self.url(path))
            .header("Accept", "text/html")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn delete_html_form(&self, path: &str, fields: &[(String, String)]) -> Response {
        self.require_local_write();
        self.client
            .delete(self.url(path))
            .header("Accept", "text/html")
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn post_html_form_from_local_client(
        &self,
        path: &str,
        fields: &[(String, String)],
        client_ip: &str,
    ) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "text/html")
            .header("X-Forwarded-For", client_ip)
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn post_html_form_from_client(
        &self,
        path: &str,
        client_ip: &str,
        fields: &[(String, String)],
    ) -> Response {
        self.post_html_form_from_local_client(path, fields, client_ip)
    }

    pub fn web_form_request(
        &self,
        method: &str,
        path: &str,
        accept: &str,
        fields: &[(String, String)],
    ) -> Response {
        self.require_local_write();
        let method = reqwest::Method::from_bytes(method.as_bytes()).expect("HTTP method");
        self.client
            .request(method, self.url(path))
            .header("Accept", accept)
            .form(fields)
            .send()
            .expect("target must respond")
    }

    pub fn post_json(&self, path: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "application/json")
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_web_json(
        &self,
        path: &str,
        csrf: &str,
        client_ip: Option<&str>,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        let request = self
            .client
            .post(self.url(path))
            .header("Accept", "application/json")
            .header("X-CSRF-Token", csrf);
        let request = match client_ip {
            Some(client_ip) => request.header("X-Forwarded-For", client_ip),
            None => request,
        };
        request.json(body).send().expect("target must respond")
    }

    pub fn post_json_from_web_client(
        &self,
        path: &str,
        client_ip: &str,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        self.client
            .post(self.url(path))
            .header("Accept", "application/json")
            .header("X-Forwarded-For", client_ip)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_json_authorized(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_json_with_header(
        &self,
        path: &str,
        token: &str,
        name: &'static str,
        value: &str,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .header(name, value)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_json_with_key(
        &self,
        path: &str,
        token: &str,
        key: &str,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .header("Idempotency-Key", key)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn post_json_from_local_client(
        &self,
        path: &str,
        token: &str,
        client_ip: &str,
        key: Option<&str>,
        body: &serde_json::Value,
    ) -> Response {
        self.require_local_write();
        let request = self
            .authorize(self.client.post(self.url(path)), Some(token))
            .header("X-Forwarded-For", client_ip);
        let request = match key {
            Some(key) => request.header("Idempotency-Key", key),
            None => request,
        };
        request.json(body).send().expect("target must respond")
    }

    pub fn post_json_if_match(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
        etag: &str,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.post(self.url(path)), Some(token))
            .header("If-Match", etag)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn patch_json(&self, path: &str, token: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.authorize(self.client.patch(self.url(path)), Some(token))
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn patch_json_without_auth(&self, path: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.client
            .patch(self.url(path))
            .header("Accept", "application/json")
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn patch_json_if_match(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
        etag: &str,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.patch(self.url(path)), Some(token))
            .header("If-Match", etag)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn put_json_if_match(
        &self,
        path: &str,
        token: &str,
        body: &serde_json::Value,
        etag: &str,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.put(self.url(path)), Some(token))
            .header("If-Match", etag)
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn put_json(&self, path: &str, token: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.authorize(self.client.put(self.url(path)), Some(token))
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn put_multipart(
        &self,
        path: &str,
        token: &str,
        form: reqwest::blocking::multipart::Form,
    ) -> Response {
        self.require_local_write();
        self.authorize(self.client.put(self.url(path)), Some(token))
            .multipart(form)
            .send()
            .expect("target must respond")
    }

    pub fn put_json_without_auth(&self, path: &str, body: &serde_json::Value) -> Response {
        self.require_local_write();
        self.client
            .put(self.url(path))
            .header("Accept", "application/json")
            .json(body)
            .send()
            .expect("target must respond")
    }

    pub fn delete_if_match(&self, path: &str, token: &str, etag: &str) -> Response {
        self.require_local_write();
        self.authorize(self.client.delete(self.url(path)), Some(token))
            .header("If-Match", etag)
            .send()
            .expect("target must respond")
    }

    fn url(&self, path: &str) -> Url {
        checked_url(&self.origin, path).expect("request path must stay within target origin")
    }

    fn authorize(&self, request: RequestBuilder, token: Option<&str>) -> RequestBuilder {
        let request = request.header("Accept", "application/json");
        match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn require_local_write(&self) {
        let local = match self.origin.host() {
            Some(Host::Domain("localhost")) => true,
            Some(Host::Ipv4(ip)) => ip.is_loopback(),
            Some(Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        assert!(local, "contract write requests require a loopback target");
    }
}

fn checked_url(origin: &Url, path: &str) -> Result<Url, String> {
    if !path.starts_with('/') || path.starts_with("//") {
        return Err("request path must start with one slash".to_string());
    }
    let url = origin.join(path).map_err(|error| error.to_string())?;
    if url.origin() != origin.origin() || !url.username().is_empty() || url.password().is_some() {
        return Err("request path changed target origin".to_string());
    }
    Ok(url)
}

pub fn fixture() -> Fixture {
    let path = env::var("CONTRACT_FIXTURE_PATH").expect("CONTRACT_FIXTURE_PATH is required");
    let metadata = fs::metadata(&path).expect("fixture file must exist");
    assert!(metadata.is_file(), "fixture path must be a file");
    let value = fs::read_to_string(Path::new(&path)).expect("read fixture");
    serde_json::from_str(&value).expect("valid fixture JSON")
}

#[cfg(test)]
mod tests {
    use super::checked_url;
    use url::Url;

    #[test]
    fn accepts_a_path_on_the_approved_origin() {
        let origin = Url::parse("http://127.0.0.1:3000").unwrap();
        let url = checked_url(&origin, "/api/v1/capabilities").unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:3000/api/v1/capabilities");
    }

    #[test]
    fn rejects_a_scheme_relative_host_before_request() {
        let origin = Url::parse("http://127.0.0.1:3000").unwrap();
        assert!(checked_url(&origin, "//other.example/api/v1/me").is_err());
    }

    #[test]
    fn rejects_an_absolute_url_before_request() {
        let origin = Url::parse("http://127.0.0.1:3000").unwrap();
        assert!(checked_url(&origin, "https://other.example/api/v1/me").is_err());
    }

    #[test]
    fn rejects_a_backslash_authority_before_request() {
        let origin = Url::parse("http://127.0.0.1:3000").unwrap();
        assert!(checked_url(&origin, "/\\other.example/api/v1/me").is_err());
    }
}
