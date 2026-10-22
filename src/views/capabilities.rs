use serde::Serialize;

use crate::models::identity::oauth::PublicMobileClient;

#[derive(Serialize)]
pub struct CapabilitiesResponse {
    data: Capabilities,
}

#[derive(Serialize)]
struct Capabilities {
    format: &'static str,
    api_version: &'static str,
    authentication: Authentication,
    administration: Administration,
    medication_pause_periods: MedicationPausePeriods,
    dose_outcomes: DoseOutcomes,
    stock_removals: StockRemovals,
    location_management: LocationManagement,
    medication_reviews: MedicationReviews,
    reports: Reports,
    invitations: Invitations,
    profile: Profile,
    portable_formats: &'static [&'static str],
    backups: Backups,
    fhir: Fhir,
    sync: Sync,
    client_tools: ClientTools,
}

#[derive(Serialize)]
struct Authentication {
    methods: &'static [&'static str],
    hosted_mobile: &'static str,
    mobile_oauth: MobileOauth,
}

#[derive(Serialize)]
struct MobileOauth {
    discovery_url: String,
    household_binding: &'static str,
    inactivity_timeout_days: i64,
    maximum_age_days: i64,
    clients: Vec<MobileClient>,
}

#[derive(Serialize)]
struct MobileClient {
    client_id: String,
    name: String,
    redirect_uris: Vec<String>,
    scopes: Vec<String>,
}

impl From<PublicMobileClient> for MobileClient {
    fn from(client: PublicMobileClient) -> Self {
        Self {
            client_id: client.client_id,
            name: client.name,
            redirect_uris: client.redirect_uris,
            scopes: client.scopes,
        }
    }
}

#[derive(Serialize)]
struct Administration {
    household: bool,
    fresh_mfa_required: bool,
    app_tokens: bool,
    audit_logs: bool,
    invitations: bool,
    person_access_grants: bool,
}

#[derive(Serialize)]
struct MedicationPausePeriods {
    supported: bool,
    reasons: &'static [&'static str],
    effective_time: &'static str,
}

#[derive(Serialize)]
struct DoseOutcomes {
    source_types: &'static [&'static str],
    max_read_days: u8,
    actions: &'static [&'static str],
    replacement_requires_version: bool,
}

#[derive(Serialize)]
struct StockRemovals {
    actions: &'static [&'static str],
    submission_id_required: bool,
    max_page_size: u8,
}

#[derive(Serialize)]
struct LocationManagement {
    actions: &'static [&'static str],
    version_required: bool,
    person_memberships: &'static [&'static str],
    memberships_online_only: bool,
}

#[derive(Serialize)]
struct MedicationReviews {
    actions: &'static [&'static str],
    version_required: bool,
    max_page_size: u8,
}

#[derive(Serialize)]
struct Reports {
    formats: &'static [&'static str],
    health_history: bool,
    medication_reviews: bool,
    selected_person_required: bool,
    health_history_max_span_days: u16,
}

#[derive(Serialize)]
struct Invitations {
    actions: &'static [&'static str],
    online_only: bool,
    acceptance_session_required: bool,
}

#[derive(Serialize)]
struct Profile {
    actions: &'static [&'static str],
    online_only: bool,
    avatar: Avatar,
}

#[derive(Serialize)]
struct Avatar {
    actions: &'static [&'static str],
    max_bytes: usize,
    content_types: &'static [&'static str],
}

#[derive(Serialize)]
struct Backups {
    encrypted_migration_bundle: bool,
    unencrypted_zip: bool,
    health_data_json: bool,
}

#[derive(Serialize)]
struct Fhir {
    version: &'static str,
    resources: &'static [&'static str],
}

#[derive(Serialize)]
struct Sync {
    portable_ids: bool,
    numeric_ids: &'static str,
    mobile_snapshot: bool,
    dry_run_import: bool,
    idempotency_keys: bool,
    etag_conflicts: bool,
    change_feed: bool,
    batch_mutations: bool,
    tombstones: bool,
    operations: &'static [SyncOperation],
    online_only_resources: &'static [&'static str],
}

#[derive(Serialize)]
struct SyncOperation {
    resource_type: &'static str,
    actions: &'static [&'static str],
}

#[derive(Serialize)]
struct ClientTools {
    cli: Cli,
    mcp_server: McpServer,
    diagnostics: &'static [&'static str],
}

#[derive(Serialize)]
struct Cli {
    supported: bool,
    status: &'static str,
    binary: &'static str,
    api_boundary: &'static str,
    distribution: &'static str,
}

#[derive(Serialize)]
struct McpServer {
    supported: bool,
    transport: &'static str,
    endpoint: &'static str,
    stdio_binary: &'static str,
    tools: &'static [&'static str],
    resources: &'static [&'static str],
}

const SYNC_OPERATIONS: &[SyncOperation] = &[
    SyncOperation {
        resource_type: "medication_take",
        actions: &["create"],
    },
    SyncOperation {
        resource_type: "medication_dose_occurrence",
        actions: &["create", "update"],
    },
    SyncOperation {
        resource_type: "medication_pause_period",
        actions: &["create", "close"],
    },
    SyncOperation {
        resource_type: "medication",
        actions: &[
            "create",
            "update",
            "delete",
            "adjust_inventory",
            "mark_as_ordered",
            "mark_as_received",
            "remove_stock",
        ],
    },
    SyncOperation {
        resource_type: "medication_dosage_option",
        actions: &["create", "update"],
    },
    SyncOperation {
        resource_type: "person",
        actions: &["create", "update"],
    },
    SyncOperation {
        resource_type: "health_event",
        actions: &["create", "update", "delete"],
    },
    SyncOperation {
        resource_type: "location",
        actions: &["create", "update", "delete"],
    },
    SyncOperation {
        resource_type: "medication_review_prompt",
        actions: &["update"],
    },
    SyncOperation {
        resource_type: "schedule",
        actions: &["create", "update", "delete", "pause", "resume"],
    },
    SyncOperation {
        resource_type: "person_medication",
        actions: &["create", "update", "delete", "pause", "resume", "reorder"],
    },
];

impl CapabilitiesResponse {
    pub fn new(
        base: &str,
        inactivity_timeout_days: i64,
        maximum_age_days: i64,
        clients: Vec<PublicMobileClient>,
    ) -> Self {
        Self {
            data: Capabilities {
                format: "medtracker.api.capabilities.v1",
                api_version: "v1",
                authentication: Authentication {
                    methods: &["oauth_bearer", "api_app_token"],
                    hosted_mobile: "rodauth_authorization_code_pkce",
                    mobile_oauth: MobileOauth {
                        discovery_url: format!(
                            "{}/.well-known/oauth-authorization-server",
                            base.trim_end_matches('/')
                        ),
                        household_binding: "account",
                        inactivity_timeout_days,
                        maximum_age_days,
                        clients: clients.into_iter().map(MobileClient::from).collect(),
                    },
                },
                administration: Administration {
                    household: true,
                    fresh_mfa_required: false,
                    app_tokens: true,
                    audit_logs: true,
                    invitations: true,
                    person_access_grants: true,
                },
                medication_pause_periods: MedicationPausePeriods {
                    supported: true,
                    reasons: &[
                        "out_of_supply",
                        "temporarily_not_needed",
                        "clinician_advice",
                        "side_effects",
                        "other",
                    ],
                    effective_time: "server_acceptance",
                },
                dose_outcomes: DoseOutcomes {
                    source_types: &["schedule", "person_medication"],
                    max_read_days: 31,
                    actions: &["not_taken", "reopen", "take"],
                    replacement_requires_version: true,
                },
                stock_removals: StockRemovals {
                    actions: &["create", "index"],
                    submission_id_required: true,
                    max_page_size: 100,
                },
                location_management: LocationManagement {
                    actions: &["create", "update", "destroy"],
                    version_required: true,
                    person_memberships: &["create", "destroy"],
                    memberships_online_only: true,
                },
                medication_reviews: MedicationReviews {
                    actions: &["index", "show", "update"],
                    version_required: true,
                    max_page_size: 100,
                },
                reports: Reports {
                    formats: &["json", "pdf"],
                    health_history: true,
                    medication_reviews: true,
                    selected_person_required: true,
                    health_history_max_span_days: 366,
                },
                invitations: Invitations {
                    actions: &["accept", "resend"],
                    online_only: true,
                    acceptance_session_required: true,
                },
                profile: Profile {
                    actions: &["show", "update"],
                    online_only: true,
                    avatar: Avatar {
                        actions: &["show", "update", "destroy"],
                        max_bytes: crate::models::profile::avatar::MAX_UPLOAD,
                        content_types: &["image/png", "image/jpeg", "image/webp"],
                    },
                },
                portable_formats: &[
                    "medtracker.portable.v1",
                    "medtracker.portable.encrypted.v1",
                    "medtracker.portable.v2",
                    "medtracker.portable.v3",
                ],
                backups: Backups {
                    encrypted_migration_bundle: true,
                    unencrypted_zip: true,
                    health_data_json: true,
                },
                fhir: Fhir {
                    version: "R4",
                    resources: &[],
                },
                sync: Sync {
                    portable_ids: true,
                    numeric_ids: "backward_compatible",
                    mobile_snapshot: true,
                    dry_run_import: true,
                    idempotency_keys: true,
                    etag_conflicts: true,
                    change_feed: true,
                    batch_mutations: true,
                    tombstones: true,
                    operations: SYNC_OPERATIONS,
                    online_only_resources: &[
                        "account",
                        "profile",
                        "avatar",
                        "invitation",
                        "household_membership",
                        "person_access_grant",
                        "location_membership",
                        "report",
                        "api_session",
                        "api_app_token",
                    ],
                },
                client_tools: ClientTools {
                    cli: Cli {
                        supported: false,
                        status: "available",
                        binary: "medtracker",
                        api_boundary: "/api/v1",
                        distribution: "github_release",
                    },
                    mcp_server: McpServer {
                        supported: false,
                        transport: "streamable_http",
                        endpoint: "/mcp",
                        stdio_binary: "medtracker-mcp",
                        tools: &[],
                        resources: &[],
                    },
                    diagnostics: &["request_id", "retry_after"],
                },
            },
        }
    }
}
