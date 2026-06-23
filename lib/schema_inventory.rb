# frozen_string_literal: true

class SchemaInventory
  HOUSEHOLD_OWNED_TABLES = %w[
    people
    locations
    location_memberships
    medications
    medication_dosage_options
    dosages
    schedules
    person_medications
    medication_takes
    notification_preferences
    household_memberships
    person_access_grants
    household_invitations
    household_invitation_grants
    security_audit_events
    versions
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
    app_settings
    barcode_catalog_entries
    nhs_dmd_barcodes
    nhs_dmd_imports
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
