# frozen_string_literal: true

class SchemaInventory
  HOUSEHOLD_OWNED_TABLES = %w[
    people
    locations
    location_memberships
    medications
    dosages
    schedules
    person_medications
    medication_takes
    notification_preferences
    health_events
    health_event_medications
    notification_events
    household_memberships
    person_access_grants
    household_invitations
    household_invitation_grants
    api_change_events
    api_idempotency_keys
    api_tombstones
    medication_review_prompts
    security_audit_events
    active_storage_attachments
  ].freeze

  GLOBAL_TABLES = %w[
    account_active_session_keys
    account_identities
    account_lockouts
    account_login_change_keys
    account_login_failures
    account_otp_keys
    account_password_reset_keys
    account_recovery_codes
    account_remember_keys
    account_verification_keys
    account_webauthn_keys
    account_webauthn_user_ids
    accounts
    active_storage_blobs
    active_storage_variant_records
    api_app_tokens
    api_oidc_nonces
    api_sessions
    app_settings
    audit_chain_heads
    audit_checkpoints
    audit_export_deliveries
    audit_ledger_entries
    audit_signing_keys
    barcode_catalog_entries
    carer_relationships
    households
    medication_review_evidence_records
    native_device_tokens
    nhs_dmd_barcodes
    nhs_dmd_imports
    oauth_applications
    oauth_grants
    platform_admins
    push_subscriptions
    support_access_sessions
    users
    versions
  ].freeze

  class << self
    def household_owned_tables
      HOUSEHOLD_OWNED_TABLES
    end

    def global_tables
      GLOBAL_TABLES
    end
  end
end
