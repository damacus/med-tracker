#[path = "support/db.rs"]
mod db;
#[path = "support/fixtures.rs"]
pub mod fixtures;
#[path = "support/harness.rs"]
mod harness;
#[path = "browser/journeys.rs"]
mod journeys;
#[path = "browser/malformed_auth_entry.rs"]
mod malformed_auth_entry;
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

#[path = "browser/dependent_creation.rs"]
mod dependent_creation;
#[path = "browser/person_editing.rs"]
mod person_editing;
#[path = "browser/review_prompt_editing.rs"]
mod review_prompt_editing;
#[path = "browser/typed_care_operations.rs"]
mod typed_care_operations;

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

#[path = "browser/dashboard_cancellation.rs"]
mod dashboard_cancellation;

#[path = "browser/report_clarity.rs"]
mod report_clarity;

#[path = "browser/security_entry.rs"]
mod security_entry;

#[path = "browser/person_owned_schedules.rs"]
mod person_owned_schedules;
#[path = "browser/personal_profile.rs"]
mod personal_profile;

#[path = "browser/report_table_layout.rs"]
mod report_table_layout;
