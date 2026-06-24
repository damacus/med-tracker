# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# This file is the source Rails uses to define your schema when running `bin/rails
# db:schema:load`. When creating a new database, `bin/rails db:schema:load` tends to
# be faster and is potentially less error prone than running all of your
# migrations from scratch. Old migrations may fail to apply correctly if those
# migrations use external dependencies or application code.
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema[8.1].define(version: 2026_06_23_090000) do
  # These are extensions that must be enabled in order to support this database
  enable_extension "citext"
  enable_extension "pg_catalog.plpgsql"
  enable_extension "pg_trgm"

  create_table "account_active_session_keys", id: false, force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.datetime "last_use", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.string "session_id", null: false
    t.index ["account_id", "session_id"], name: "index_account_active_session_keys_on_account_id_and_session_id", unique: true
    t.index ["account_id"], name: "index_account_active_session_keys_on_account_id"
  end

  create_table "account_identities", force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.string "provider", null: false
    t.string "uid", null: false
    t.datetime "updated_at", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.index ["account_id"], name: "index_account_identities_on_account_id"
    t.index ["provider", "uid"], name: "index_account_identities_on_provider_and_uid", unique: true
  end

  create_table "account_lockouts", primary_key: "account_id", force: :cascade do |t|
    t.datetime "created_at"
    t.datetime "deadline", null: false
    t.datetime "email_last_sent"
    t.string "key", null: false
    t.datetime "updated_at"
    t.index ["account_id"], name: "index_account_lockouts_on_account_id"
  end

  create_table "account_login_change_keys", id: false, force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at"
    t.datetime "deadline", null: false
    t.string "key", null: false
    t.string "login", null: false
    t.datetime "updated_at"
    t.index ["account_id"], name: "index_account_login_change_keys_on_account_id"
  end

  create_table "account_login_failures", primary_key: "account_id", force: :cascade do |t|
    t.datetime "created_at"
    t.integer "number", default: 1, null: false
    t.datetime "updated_at"
    t.index ["account_id"], name: "index_account_login_failures_on_account_id"
  end

  create_table "account_otp_keys", force: :cascade do |t|
    t.string "key", null: false
    t.datetime "last_use", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.integer "num_failures", default: 0, null: false
  end

  create_table "account_password_reset_keys", id: false, force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at"
    t.datetime "deadline", null: false
    t.datetime "email_last_sent", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.string "key", null: false
    t.datetime "updated_at"
    t.index ["account_id"], name: "index_account_password_reset_keys_on_account_id"
  end

  create_table "account_recovery_codes", primary_key: ["id", "code"], force: :cascade do |t|
    t.string "code", null: false
    t.bigint "id", null: false
  end

  create_table "account_remember_keys", id: false, force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at"
    t.datetime "deadline", null: false
    t.string "key", null: false
    t.datetime "updated_at"
    t.index ["account_id"], name: "index_account_remember_keys_on_account_id"
  end

  create_table "account_verification_keys", id: false, force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at"
    t.datetime "email_last_sent", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.string "key", null: false
    t.datetime "requested_at", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.datetime "updated_at"
    t.index ["account_id"], name: "index_account_verification_keys_on_account_id"
  end

  create_table "account_webauthn_keys", force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.datetime "last_use"
    t.string "nickname"
    t.string "public_key", null: false
    t.integer "sign_count", default: 0, null: false
    t.datetime "updated_at", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.string "webauthn_id", null: false
    t.index ["account_id"], name: "index_account_webauthn_keys_on_account_id"
    t.index ["webauthn_id", "account_id"], name: "index_account_webauthn_keys_on_webauthn_id_and_account_id", unique: true
  end

  create_table "account_webauthn_user_ids", force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.datetime "updated_at", default: -> { "CURRENT_TIMESTAMP" }, null: false
    t.string "webauthn_id", null: false
    t.index ["account_id"], name: "index_account_webauthn_user_ids_on_account_id"
    t.index ["webauthn_id"], name: "index_account_webauthn_user_ids_on_webauthn_id", unique: true
  end

  create_table "accounts", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.citext "email", null: false
    t.string "password_hash"
    t.jsonb "preferences", default: {}, null: false
    t.integer "status", default: 1, null: false
    t.datetime "updated_at", null: false
    t.index ["email"], name: "index_accounts_on_email", unique: true, where: "(status = ANY (ARRAY[1, 2]))"
    t.index ["preferences"], name: "index_accounts_on_preferences", using: :gin
  end

  create_table "active_storage_attachments", force: :cascade do |t|
    t.bigint "blob_id", null: false
    t.datetime "created_at", null: false
    t.bigint "household_id"
    t.string "name", null: false
    t.bigint "record_id", null: false
    t.string "record_type", null: false
    t.index ["blob_id"], name: "index_active_storage_attachments_on_blob_id"
    t.index ["household_id"], name: "index_active_storage_attachments_on_household_id"
    t.index ["id", "household_id"], name: "index_active_storage_attachments_on_id_and_household_id", unique: true
    t.index ["record_type", "record_id", "name", "blob_id"], name: "index_active_storage_attachments_uniqueness", unique: true
  end

  create_table "active_storage_blobs", force: :cascade do |t|
    t.bigint "byte_size", null: false
    t.string "checksum"
    t.string "content_type"
    t.datetime "created_at", null: false
    t.string "filename", null: false
    t.string "key", null: false
    t.text "metadata"
    t.string "service_name", null: false
    t.index ["key"], name: "index_active_storage_blobs_on_key", unique: true
  end

  create_table "active_storage_variant_records", force: :cascade do |t|
    t.bigint "blob_id", null: false
    t.string "variation_digest", null: false
    t.index ["blob_id", "variation_digest"], name: "index_active_storage_variant_records_uniqueness", unique: true
  end

  create_table "api_app_tokens", force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at", null: false
    t.bigint "household_membership_id", null: false
    t.datetime "last_used_at", null: false
    t.string "name", null: false
    t.integer "permissions_version", default: 1, null: false
    t.datetime "revoked_at"
    t.string "token_digest", null: false
    t.datetime "updated_at", null: false
    t.index ["account_id"], name: "index_api_app_tokens_on_account_id"
    t.index ["household_membership_id", "revoked_at"], name: "index_api_app_tokens_on_membership_and_revoked_at"
    t.index ["household_membership_id"], name: "index_api_app_tokens_on_household_membership_id"
    t.index ["token_digest"], name: "index_api_app_tokens_on_token_digest", unique: true
  end

  create_table "api_sessions", force: :cascade do |t|
    t.datetime "access_expires_at", null: false
    t.string "access_token_digest", null: false
    t.bigint "account_id", null: false
    t.datetime "created_at", null: false
    t.string "device_name"
    t.bigint "household_membership_id"
    t.datetime "last_used_at", null: false
    t.integer "permissions_version", default: 1, null: false
    t.datetime "refresh_expires_at", null: false
    t.string "refresh_token_digest", null: false
    t.datetime "revoked_at"
    t.datetime "updated_at", null: false
    t.string "user_agent"
    t.index ["access_token_digest"], name: "index_api_sessions_on_access_token_digest", unique: true
    t.index ["account_id"], name: "index_api_sessions_on_account_id"
    t.index ["household_membership_id", "revoked_at"], name: "index_api_sessions_on_membership_and_revoked_at"
    t.index ["household_membership_id"], name: "index_api_sessions_on_household_membership_id"
    t.index ["refresh_token_digest"], name: "index_api_sessions_on_refresh_token_digest", unique: true
    t.index ["revoked_at"], name: "index_api_sessions_on_revoked_at"
  end

  create_table "app_settings", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.boolean "invite_only", default: false, null: false
    t.datetime "updated_at", null: false
  end

  create_table "barcode_catalog_entries", force: :cascade do |t|
    t.string "code"
    t.string "concept_class"
    t.datetime "created_at", null: false
    t.string "display", null: false
    t.string "gtin", null: false
    t.string "source", null: false
    t.string "system"
    t.datetime "updated_at", null: false
    t.index ["gtin"], name: "index_barcode_catalog_entries_on_gtin"
    t.index ["source", "gtin"], name: "index_barcode_catalog_entries_on_source_and_gtin", unique: true
  end

  create_table "carer_relationships", force: :cascade do |t|
    t.boolean "active", default: true, null: false
    t.bigint "carer_id", null: false
    t.datetime "created_at", null: false
    t.bigint "patient_id", null: false
    t.string "relationship_type"
    t.datetime "updated_at", null: false
    t.index ["active"], name: "index_carer_relationships_on_active"
    t.index ["carer_id", "patient_id"], name: "index_carer_relationships_on_carer_id_and_patient_id", unique: true
    t.index ["carer_id"], name: "index_carer_relationships_on_carer_id"
    t.index ["patient_id"], name: "index_carer_relationships_on_patient_id"
  end

  create_table "dosages", force: :cascade do |t|
    t.decimal "amount"
    t.datetime "created_at", null: false
    t.decimal "current_supply", precision: 10, scale: 2
    t.integer "default_dose_cycle"
    t.boolean "default_for_adults", default: false, null: false
    t.boolean "default_for_children", default: false, null: false
    t.integer "default_max_daily_doses"
    t.decimal "default_min_hours_between_doses", precision: 4, scale: 1
    t.string "description"
    t.string "frequency"
    t.bigint "household_id"
    t.bigint "medication_id", null: false
    t.decimal "reorder_threshold", precision: 10, scale: 2
    t.string "unit"
    t.datetime "updated_at", null: false
    t.index ["household_id"], name: "index_dosages_on_household_id"
    t.index ["id", "household_id"], name: "index_dosages_on_id_and_household_id", unique: true
    t.index ["medication_id"], name: "index_dosages_on_medication_id"
    t.index ["medication_id"], name: "index_dosages_one_adult_default", unique: true, where: "(default_for_adults = true)"
    t.index ["medication_id"], name: "index_dosages_one_child_default", unique: true, where: "(default_for_children = true)"
  end

  create_table "household_invitation_grants", force: :cascade do |t|
    t.string "access_level", null: false
    t.datetime "created_at", null: false
    t.datetime "expires_at"
    t.bigint "household_id", null: false
    t.bigint "household_invitation_id", null: false
    t.bigint "person_id", null: false
    t.string "relationship_type", null: false
    t.datetime "updated_at", null: false
    t.index ["household_id"], name: "index_household_invitation_grants_on_household_id"
    t.index ["household_invitation_id"], name: "index_household_invitation_grants_on_household_invitation_id"
    t.index ["id", "household_id"], name: "index_household_invitation_grants_on_id_and_household_id", unique: true
    t.index ["person_id"], name: "index_household_invitation_grants_on_person_id"
  end

  create_table "household_invitations", force: :cascade do |t|
    t.datetime "accepted_at"
    t.datetime "created_at", null: false
    t.citext "email", null: false
    t.datetime "expires_at", null: false
    t.bigint "household_id", null: false
    t.bigint "invited_by_membership_id", null: false
    t.string "membership_role", null: false
    t.datetime "revoked_at"
    t.string "token_digest", null: false
    t.datetime "updated_at", null: false
    t.index ["household_id", "email"], name: "index_household_invitations_on_household_id_and_email", unique: true, where: "((accepted_at IS NULL) AND (revoked_at IS NULL))"
    t.index ["household_id"], name: "index_household_invitations_on_household_id"
    t.index ["id", "household_id"], name: "index_household_invitations_on_id_and_household_id", unique: true
    t.index ["invited_by_membership_id"], name: "index_household_invitations_on_invited_by_membership_id"
    t.index ["token_digest"], name: "index_household_invitations_on_token_digest", unique: true
  end

  create_table "household_memberships", force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at", null: false
    t.bigint "household_id", null: false
    t.datetime "joined_at", null: false
    t.integer "permissions_version", default: 1, null: false
    t.bigint "person_id"
    t.datetime "revoked_at"
    t.string "role", default: "member", null: false
    t.string "status", default: "active", null: false
    t.datetime "updated_at", null: false
    t.index ["account_id"], name: "index_household_memberships_on_account_id"
    t.index ["household_id", "account_id"], name: "index_household_memberships_on_household_id_and_account_id", unique: true
    t.index ["household_id"], name: "index_household_memberships_on_household_id"
    t.index ["id", "household_id"], name: "index_household_memberships_on_id_and_household_id", unique: true
    t.index ["person_id"], name: "index_household_memberships_on_person_id", unique: true, where: "(person_id IS NOT NULL)"
  end

  create_table "households", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.bigint "created_by_account_id"
    t.string "name", null: false
    t.string "slug", null: false
    t.string "status", default: "active", null: false
    t.string "subscription_plan", default: "free", null: false
    t.string "timezone", null: false
    t.datetime "updated_at", null: false
    t.index ["created_by_account_id"], name: "index_households_on_created_by_account_id"
    t.index ["slug"], name: "index_households_on_slug", unique: true
  end

  create_table "location_memberships", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.bigint "household_id"
    t.bigint "location_id", null: false
    t.bigint "person_id", null: false
    t.datetime "updated_at", null: false
    t.index ["household_id"], name: "index_location_memberships_on_household_id"
    t.index ["id", "household_id"], name: "index_location_memberships_on_id_and_household_id", unique: true
    t.index ["location_id"], name: "index_location_memberships_on_location_id"
    t.index ["person_id", "location_id"], name: "index_location_memberships_on_person_id_and_location_id", unique: true
    t.index ["person_id"], name: "index_location_memberships_on_person_id"
  end

  create_table "locations", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.text "description"
    t.bigint "household_id"
    t.string "name", null: false
    t.datetime "updated_at", null: false
    t.index "household_id, lower((name)::text)", name: "index_locations_on_household_id_and_lower_name", unique: true, where: "(household_id IS NOT NULL)"
    t.index ["household_id"], name: "index_locations_on_household_id"
    t.index ["id", "household_id"], name: "index_locations_on_id_and_household_id", unique: true
    t.index ["name"], name: "index_locations_on_name_trigram", opclass: :gin_trgm_ops, using: :gin
  end

  create_table "medication_takes", force: :cascade do |t|
    t.string "client_uuid"
    t.datetime "created_at", null: false
    t.decimal "dose_amount", precision: 10, scale: 2
    t.string "dose_unit"
    t.bigint "household_id"
    t.bigint "person_medication_id"
    t.bigint "schedule_id"
    t.datetime "taken_at"
    t.bigint "taken_from_location_id"
    t.bigint "taken_from_medication_id"
    t.datetime "updated_at", null: false
    t.index ["client_uuid"], name: "index_medication_takes_on_client_uuid", unique: true, where: "(client_uuid IS NOT NULL)"
    t.index ["household_id"], name: "index_medication_takes_on_household_id"
    t.index ["id", "household_id"], name: "index_medication_takes_on_id_and_household_id", unique: true
    t.index ["person_medication_id"], name: "index_medication_takes_on_person_medication_id"
    t.index ["schedule_id"], name: "index_medication_takes_on_schedule_id"
    t.index ["taken_at"], name: "index_medication_takes_on_taken_at"
    t.index ["taken_from_location_id"], name: "index_medication_takes_on_taken_from_location_id"
    t.index ["taken_from_medication_id"], name: "index_medication_takes_on_taken_from_medication_id"
  end

  create_table "medications", force: :cascade do |t|
    t.string "barcode"
    t.string "category"
    t.datetime "created_at", null: false
    t.decimal "current_supply", precision: 10, scale: 2
    t.jsonb "default_schedule_config", default: {}, null: false
    t.integer "default_schedule_type", default: 1, null: false
    t.text "description"
    t.string "dmd_code"
    t.string "dmd_concept_class"
    t.string "dmd_system"
    t.float "dosage_amount"
    t.string "dosage_unit"
    t.date "expiry_date"
    t.string "friendly_name"
    t.bigint "household_id"
    t.bigint "location_id", null: false
    t.string "name"
    t.datetime "ordered_at"
    t.integer "reorder_status"
    t.decimal "reorder_threshold", precision: 10, scale: 2, default: "10.0", null: false
    t.datetime "reordered_at"
    t.decimal "supply_at_last_restock", precision: 10, scale: 2
    t.datetime "updated_at", null: false
    t.text "warnings"
    t.index ["barcode"], name: "index_medications_on_barcode", unique: true, where: "((barcode IS NOT NULL) AND ((barcode)::text <> ''::text))"
    t.index ["barcode"], name: "index_medications_on_barcode_trigram", opclass: :gin_trgm_ops, using: :gin
    t.index ["category"], name: "index_medications_on_category_trigram", opclass: :gin_trgm_ops, using: :gin
    t.index ["default_schedule_type"], name: "index_medications_on_default_schedule_type"
    t.index ["dmd_code"], name: "index_medications_on_dmd_code"
    t.index ["dmd_code"], name: "index_medications_on_dmd_code_trigram", opclass: :gin_trgm_ops, using: :gin
    t.index ["household_id"], name: "index_medications_on_household_id"
    t.index ["id", "household_id"], name: "index_medications_on_id_and_household_id", unique: true
    t.index ["location_id"], name: "index_medications_on_location_id"
    t.index ["name"], name: "index_medications_on_name_trigram", opclass: :gin_trgm_ops, using: :gin
  end

  create_table "native_device_tokens", force: :cascade do |t|
    t.bigint "account_id", null: false
    t.datetime "created_at", null: false
    t.string "device_token", null: false
    t.string "platform", null: false
    t.datetime "updated_at", null: false
    t.string "user_agent"
    t.index ["account_id", "platform"], name: "index_native_device_tokens_on_account_id_and_platform"
    t.index ["account_id"], name: "index_native_device_tokens_on_account_id"
    t.index ["device_token"], name: "index_native_device_tokens_on_device_token", unique: true
  end

  create_table "nhs_dmd_barcodes", force: :cascade do |t|
    t.string "code", null: false
    t.string "concept_class"
    t.datetime "created_at", null: false
    t.string "display", null: false
    t.string "gtin", null: false
    t.string "system", default: "https://dmd.nhs.uk", null: false
    t.datetime "updated_at", null: false
    t.string "vmp_name"
    t.index ["code"], name: "index_nhs_dmd_barcodes_on_code"
    t.index ["gtin"], name: "index_nhs_dmd_barcodes_on_gtin", unique: true
  end

  create_table "nhs_dmd_imports", force: :cascade do |t|
    t.string "archive_path"
    t.datetime "completed_at"
    t.datetime "created_at", null: false
    t.integer "created_count", default: 0, null: false
    t.text "error_message"
    t.integer "imported_count", default: 0, null: false
    t.text "log"
    t.integer "processed_records", default: 0, null: false
    t.integer "skipped_count", default: 0, null: false
    t.integer "skipped_expired_count", default: 0, null: false
    t.integer "skipped_invalid_count", default: 0, null: false
    t.integer "skipped_missing_name_count", default: 0, null: false
    t.datetime "started_at"
    t.integer "status", default: 0, null: false
    t.integer "total_records", default: 0, null: false
    t.integer "unchanged_count", default: 0, null: false
    t.datetime "updated_at", null: false
    t.integer "updated_count", default: 0, null: false
    t.string "uploaded_filename", null: false
  end

  create_table "notification_preferences", force: :cascade do |t|
    t.time "afternoon_time", default: "2000-01-01 14:00:00"
    t.datetime "created_at", null: false
    t.boolean "enabled", default: true, null: false
    t.time "evening_time", default: "2000-01-01 18:00:00"
    t.bigint "household_id"
    t.time "morning_time", default: "2000-01-01 08:00:00"
    t.time "night_time", default: "2000-01-01 22:00:00"
    t.bigint "person_id", null: false
    t.datetime "updated_at", null: false
    t.index ["household_id"], name: "index_notification_preferences_on_household_id"
    t.index ["id", "household_id"], name: "index_notification_preferences_on_id_and_household_id", unique: true
    t.index ["person_id"], name: "index_notification_preferences_on_person_id", unique: true
  end

  create_table "people", force: :cascade do |t|
    t.bigint "account_id"
    t.datetime "created_at", null: false
    t.date "date_of_birth"
    t.string "email"
    t.boolean "has_capacity", default: true, null: false
    t.bigint "household_id"
    t.string "name", null: false
    t.integer "person_type", default: 0, null: false
    t.string "professional_title"
    t.datetime "updated_at", null: false
    t.index ["account_id"], name: "index_people_on_account_id"
    t.index ["email"], name: "index_people_on_email_present_unique", unique: true, where: "((email IS NOT NULL) AND (btrim((email)::text) <> ''::text))"
    t.index ["household_id", "account_id"], name: "index_people_on_household_id_and_account_id", unique: true, where: "((household_id IS NOT NULL) AND (account_id IS NOT NULL))"
    t.index ["household_id"], name: "index_people_on_household_id"
    t.index ["id", "household_id"], name: "index_people_on_id_and_household_id", unique: true
    t.index ["name"], name: "index_people_on_name_trigram", opclass: :gin_trgm_ops, using: :gin
    t.index ["person_type"], name: "index_people_on_person_type"
  end

  create_table "person_access_grants", force: :cascade do |t|
    t.string "access_level", null: false
    t.datetime "created_at", null: false
    t.datetime "expires_at"
    t.bigint "granted_by_membership_id"
    t.bigint "household_id", null: false
    t.bigint "household_membership_id", null: false
    t.bigint "person_id", null: false
    t.string "relationship_type", null: false
    t.datetime "revoked_at"
    t.datetime "updated_at", null: false
    t.index ["granted_by_membership_id"], name: "index_person_access_grants_on_granted_by_membership_id"
    t.index ["household_id"], name: "index_person_access_grants_on_household_id"
    t.index ["household_membership_id", "person_id"], name: "idx_on_household_membership_id_person_id_6ddc5a2882", unique: true, where: "(revoked_at IS NULL)"
    t.index ["household_membership_id"], name: "index_person_access_grants_on_household_membership_id"
    t.index ["id", "household_id"], name: "index_person_access_grants_on_id_and_household_id", unique: true
    t.index ["person_id"], name: "index_person_access_grants_on_person_id"
  end

  create_table "person_medications", force: :cascade do |t|
    t.integer "administration_kind", default: 1, null: false
    t.datetime "created_at", null: false
    t.decimal "dose_amount", precision: 10, scale: 2
    t.integer "dose_cycle"
    t.string "dose_unit"
    t.bigint "household_id"
    t.integer "max_daily_doses"
    t.bigint "medication_id", null: false
    t.integer "min_hours_between_doses"
    t.text "notes"
    t.bigint "person_id", null: false
    t.integer "position", null: false
    t.bigint "source_dosage_option_id"
    t.datetime "updated_at", null: false
    t.index ["administration_kind"], name: "index_person_medications_on_administration_kind"
    t.index ["household_id"], name: "index_person_medications_on_household_id"
    t.index ["id", "household_id"], name: "index_person_medications_on_id_and_household_id", unique: true
    t.index ["medication_id"], name: "index_person_medications_on_medication_id"
    t.index ["person_id", "medication_id"], name: "index_person_medications_on_person_id_and_medication_id", unique: true
    t.index ["person_id", "position"], name: "index_person_medications_on_person_id_and_position"
    t.index ["person_id"], name: "index_person_medications_on_person_id"
    t.index ["source_dosage_option_id"], name: "index_person_medications_on_source_dosage_option_id"
  end

  create_table "push_subscriptions", force: :cascade do |t|
    t.bigint "account_id", null: false
    t.string "auth", null: false
    t.datetime "created_at", null: false
    t.string "endpoint", null: false
    t.string "p256dh", null: false
    t.datetime "updated_at", null: false
    t.string "user_agent"
    t.index ["account_id"], name: "index_push_subscriptions_on_account_id"
    t.index ["endpoint"], name: "index_push_subscriptions_on_endpoint", unique: true
  end

  create_table "schedules", force: :cascade do |t|
    t.boolean "active", default: true, null: false
    t.datetime "created_at", null: false
    t.decimal "dose_amount", precision: 10, scale: 2
    t.integer "dose_cycle"
    t.string "dose_unit"
    t.date "end_date"
    t.string "frequency"
    t.bigint "household_id"
    t.integer "max_daily_doses", default: 4
    t.bigint "medication_id", null: false
    t.integer "min_hours_between_doses"
    t.text "notes"
    t.bigint "person_id", null: false
    t.jsonb "schedule_config", default: {}, null: false
    t.integer "schedule_type", default: 0, null: false
    t.bigint "source_dosage_option_id"
    t.date "start_date"
    t.datetime "updated_at", null: false
    t.index ["active"], name: "index_schedules_on_active"
    t.index ["household_id"], name: "index_schedules_on_household_id"
    t.index ["id", "household_id"], name: "index_schedules_on_id_and_household_id", unique: true
    t.index ["medication_id"], name: "index_schedules_on_medication_id"
    t.index ["person_id"], name: "index_schedules_on_person_id"
    t.index ["schedule_config"], name: "index_schedules_on_schedule_config", using: :gin
    t.index ["schedule_type"], name: "index_schedules_on_schedule_type"
    t.index ["source_dosage_option_id"], name: "index_schedules_on_source_dosage_option_id"
  end

  create_table "security_audit_events", force: :cascade do |t|
    t.bigint "actor_account_id"
    t.bigint "actor_membership_id"
    t.datetime "created_at", null: false
    t.string "event_type", null: false
    t.bigint "household_id"
    t.string "ip"
    t.jsonb "metadata", default: {}, null: false
    t.string "request_id"
    t.datetime "updated_at", null: false
    t.index ["actor_account_id"], name: "index_security_audit_events_on_actor_account_id"
    t.index ["actor_membership_id"], name: "index_security_audit_events_on_actor_membership_id"
    t.index ["event_type"], name: "index_security_audit_events_on_event_type"
    t.index ["household_id", "created_at"], name: "index_security_audit_events_on_household_id_and_created_at"
    t.index ["household_id"], name: "index_security_audit_events_on_household_id"
    t.index ["id", "household_id"], name: "index_security_audit_events_on_id_and_household_id", unique: true
  end

  create_table "users", force: :cascade do |t|
    t.boolean "active", default: true, null: false
    t.datetime "created_at", null: false
    t.string "email_address", null: false
    t.string "password_digest"
    t.bigint "person_id", null: false
    t.datetime "updated_at", null: false
    t.index ["email_address"], name: "index_users_on_email_address", unique: true
    t.index ["person_id"], name: "index_users_on_person_id", unique: true
  end

  create_table "versions", force: :cascade do |t|
    t.bigint "actor_membership_id"
    t.datetime "created_at"
    t.string "event", null: false
    t.bigint "household_id"
    t.string "ip"
    t.bigint "item_id", null: false
    t.string "item_type", null: false
    t.text "object"
    t.string "request_id"
    t.string "whodunnit"
    t.index ["actor_membership_id"], name: "index_versions_on_actor_membership_id"
    t.index ["created_at"], name: "index_versions_on_created_at"
    t.index ["event"], name: "index_versions_on_event"
    t.index ["household_id"], name: "index_versions_on_household_id"
    t.index ["item_type", "item_id"], name: "index_versions_on_item_type_and_item_id"
    t.index ["request_id"], name: "index_versions_on_request_id"
    t.index ["whodunnit"], name: "index_versions_on_whodunnit"
  end

  add_foreign_key "account_active_session_keys", "accounts"
  add_foreign_key "account_identities", "accounts", on_delete: :cascade
  add_foreign_key "account_lockouts", "accounts"
  add_foreign_key "account_login_change_keys", "accounts"
  add_foreign_key "account_login_failures", "accounts"
  add_foreign_key "account_otp_keys", "accounts", column: "id"
  add_foreign_key "account_password_reset_keys", "accounts"
  add_foreign_key "account_recovery_codes", "accounts", column: "id"
  add_foreign_key "account_remember_keys", "accounts"
  add_foreign_key "account_verification_keys", "accounts"
  add_foreign_key "account_webauthn_keys", "accounts", on_delete: :cascade
  add_foreign_key "account_webauthn_user_ids", "accounts", on_delete: :cascade
  add_foreign_key "active_storage_attachments", "active_storage_blobs", column: "blob_id"
  add_foreign_key "active_storage_attachments", "households"
  add_foreign_key "active_storage_variant_records", "active_storage_blobs", column: "blob_id"
  add_foreign_key "api_app_tokens", "accounts"
  add_foreign_key "api_app_tokens", "household_memberships"
  add_foreign_key "api_sessions", "accounts"
  add_foreign_key "api_sessions", "household_memberships"
  add_foreign_key "carer_relationships", "people", column: "carer_id", deferrable: :deferred
  add_foreign_key "carer_relationships", "people", column: "patient_id", deferrable: :deferred
  add_foreign_key "dosages", "households"
  add_foreign_key "dosages", "medications", column: ["medication_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_dosages_medication_id_household"
  add_foreign_key "dosages", "medications", deferrable: :deferred
  add_foreign_key "household_invitation_grants", "household_invitations"
  add_foreign_key "household_invitation_grants", "household_invitations", column: ["household_invitation_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_household_invitation_grants_invitation_household"
  add_foreign_key "household_invitation_grants", "households"
  add_foreign_key "household_invitation_grants", "people"
  add_foreign_key "household_invitation_grants", "people", column: ["person_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_household_invitation_grants_person_household"
  add_foreign_key "household_invitations", "household_memberships", column: "invited_by_membership_id"
  add_foreign_key "household_invitations", "households"
  add_foreign_key "household_memberships", "accounts"
  add_foreign_key "household_memberships", "households"
  add_foreign_key "household_memberships", "people"
  add_foreign_key "household_memberships", "people", column: ["person_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_household_memberships_person_id_household"
  add_foreign_key "households", "accounts", column: "created_by_account_id"
  add_foreign_key "location_memberships", "households"
  add_foreign_key "location_memberships", "locations", column: ["location_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_location_memberships_location_id_household"
  add_foreign_key "location_memberships", "locations", deferrable: :deferred
  add_foreign_key "location_memberships", "people", column: ["person_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_location_memberships_person_id_household"
  add_foreign_key "location_memberships", "people", deferrable: :deferred
  add_foreign_key "locations", "households"
  add_foreign_key "medication_takes", "households"
  add_foreign_key "medication_takes", "locations", column: "taken_from_location_id"
  add_foreign_key "medication_takes", "locations", column: ["taken_from_location_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_medication_takes_taken_from_location_id_household"
  add_foreign_key "medication_takes", "medications", column: "taken_from_medication_id"
  add_foreign_key "medication_takes", "medications", column: ["taken_from_medication_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_medication_takes_taken_from_medication_id_household"
  add_foreign_key "medication_takes", "person_medications", column: ["person_medication_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_medication_takes_person_medication_id_household"
  add_foreign_key "medication_takes", "person_medications", deferrable: :deferred
  add_foreign_key "medication_takes", "schedules", column: ["schedule_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_medication_takes_schedule_id_household"
  add_foreign_key "medication_takes", "schedules", deferrable: :deferred
  add_foreign_key "medications", "households"
  add_foreign_key "medications", "locations", column: ["location_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_medications_location_id_household"
  add_foreign_key "medications", "locations", deferrable: :deferred
  add_foreign_key "native_device_tokens", "accounts"
  add_foreign_key "notification_preferences", "households"
  add_foreign_key "notification_preferences", "people", column: ["person_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_notification_preferences_person_id_household"
  add_foreign_key "notification_preferences", "people", deferrable: :deferred
  add_foreign_key "people", "accounts", deferrable: :deferred
  add_foreign_key "people", "households"
  add_foreign_key "person_access_grants", "household_memberships"
  add_foreign_key "person_access_grants", "household_memberships", column: "granted_by_membership_id"
  add_foreign_key "person_access_grants", "household_memberships", column: ["granted_by_membership_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_person_access_grants_granted_by_membership_id_household"
  add_foreign_key "person_access_grants", "household_memberships", column: ["household_membership_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_person_access_grants_household_membership_id_household"
  add_foreign_key "person_access_grants", "households"
  add_foreign_key "person_access_grants", "people"
  add_foreign_key "person_access_grants", "people", column: ["person_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_person_access_grants_person_id_household"
  add_foreign_key "person_medications", "dosages", column: "source_dosage_option_id"
  add_foreign_key "person_medications", "dosages", column: ["source_dosage_option_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_person_medications_source_dosage_option_id_household"
  add_foreign_key "person_medications", "households"
  add_foreign_key "person_medications", "medications", column: ["medication_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_person_medications_medication_id_household"
  add_foreign_key "person_medications", "medications", deferrable: :deferred
  add_foreign_key "person_medications", "people", column: ["person_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_person_medications_person_id_household"
  add_foreign_key "person_medications", "people", deferrable: :deferred
  add_foreign_key "push_subscriptions", "accounts", deferrable: :deferred
  add_foreign_key "schedules", "dosages", column: "source_dosage_option_id"
  add_foreign_key "schedules", "dosages", column: ["source_dosage_option_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_schedules_source_dosage_option_id_household"
  add_foreign_key "schedules", "households"
  add_foreign_key "schedules", "medications", column: ["medication_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_schedules_medication_id_household"
  add_foreign_key "schedules", "medications", deferrable: :deferred
  add_foreign_key "schedules", "people", column: ["person_id", "household_id"], primary_key: ["id", "household_id"], name: "fk_schedules_person_id_household"
  add_foreign_key "schedules", "people", deferrable: :deferred
  add_foreign_key "security_audit_events", "accounts", column: "actor_account_id"
  add_foreign_key "security_audit_events", "household_memberships", column: "actor_membership_id"
  add_foreign_key "security_audit_events", "households"
  add_foreign_key "users", "people", deferrable: :deferred
  add_foreign_key "versions", "household_memberships", column: "actor_membership_id"
  add_foreign_key "versions", "households"
end
