#[path = "support/db.rs"]
mod db;
#[path = "support/fixtures.rs"]
pub mod fixtures;
#[path = "support/harness.rs"]
mod harness;
#[path = "browser/journeys.rs"]
mod journeys;
#[path = "browser/native_auth.rs"]
mod native_auth;
#[path = "browser/native_auth_forms.rs"]
mod native_auth_forms;
#[path = "browser/oauth_consent.rs"]
mod oauth_consent;
#[path = "support/template.rs"]
mod template;

#[path = "browser/shared_layout.rs"]
mod shared_layout;

#[path = "browser/people_polish.rs"]
mod people_polish;

#[path = "browser/administration_polish.rs"]
mod administration_polish;

#[path = "browser/member_details.rs"]
mod member_details;

#[path = "browser/invitation_management.rs"]
mod invitation_management;

#[path = "browser/bulk_members.rs"]
mod bulk_members;
#[path = "browser/polish_followups.rs"]
mod polish_followups;

#[path = "browser/storage_location_deletion.rs"]
mod storage_location_deletion;

#[path = "browser/accessibility_level_a.rs"]
mod accessibility_level_a;

#[path = "browser/accessibility_journeys.rs"]
mod accessibility_journeys;

#[path = "browser/login_layout.rs"]
mod login_layout;
