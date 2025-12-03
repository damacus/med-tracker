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

ActiveRecord::Schema[8.1].define(version: 2025_12_03_164512) do
  # These are extensions that must be enabled in order to support this database
  enable_extension "citext"
  enable_extension "pg_catalog.plpgsql"

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
    t.datetime "created_at", null: false
    t.string "provider", null: false
    t.string "uid", null: false
    t.datetime "updated_at", null: false
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

  create_table "accounts", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.citext "email", null: false
    t.string "password_hash"
    t.integer "status", default: 1, null: false
    t.datetime "updated_at", null: false
    t.index ["email"], name: "index_accounts_on_email", unique: true, where: "(status = ANY (ARRAY[1, 2]))"
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
    t.string "description"
    t.string "frequency"
    t.bigint "medicine_id", null: false
    t.string "unit"
    t.datetime "updated_at", null: false
    t.index ["medicine_id"], name: "index_dosages_on_medicine_id"
  end

  create_table "medication_takes", force: :cascade do |t|
    t.decimal "amount_ml"
    t.datetime "created_at", null: false
    t.bigint "person_medicine_id"
    t.bigint "prescription_id"
    t.datetime "taken_at"
    t.datetime "updated_at", null: false
    t.index ["person_medicine_id"], name: "index_medication_takes_on_person_medicine_id"
    t.index ["prescription_id"], name: "index_medication_takes_on_prescription_id"
    t.index ["taken_at"], name: "index_medication_takes_on_taken_at"
  end

  create_table "medicines", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "current_supply"
    t.text "description"
    t.float "dosage_amount"
    t.string "dosage_unit"
    t.date "expiry_date"
    t.string "name"
    t.integer "reorder_threshold", default: 10, null: false
    t.integer "stock"
    t.datetime "updated_at", null: false
    t.text "warnings"
  end

  create_table "people", force: :cascade do |t|
    t.bigint "account_id"
    t.datetime "created_at", null: false
    t.date "date_of_birth"
    t.string "email"
    t.boolean "has_capacity", default: true, null: false
    t.string "name", null: false
    t.integer "person_type", default: 0, null: false
    t.datetime "updated_at", null: false
    t.index ["account_id"], name: "index_people_on_account_id"
    t.index ["email"], name: "index_people_on_email", unique: true
    t.index ["person_type"], name: "index_people_on_person_type"
  end

  create_table "person_medicines", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "max_daily_doses"
    t.bigint "medicine_id", null: false
    t.integer "min_hours_between_doses"
    t.text "notes"
    t.bigint "person_id", null: false
    t.datetime "updated_at", null: false
    t.index ["medicine_id"], name: "index_person_medicines_on_medicine_id"
    t.index ["person_id", "medicine_id"], name: "index_person_medicines_on_person_id_and_medicine_id", unique: true
    t.index ["person_id"], name: "index_person_medicines_on_person_id"
  end

  create_table "prescriptions", force: :cascade do |t|
    t.boolean "active", default: true, null: false
    t.datetime "created_at", null: false
    t.bigint "dosage_id", null: false
    t.integer "dose_cycle"
    t.date "end_date"
    t.string "frequency"
    t.integer "max_daily_doses", default: 4
    t.bigint "medicine_id", null: false
    t.integer "min_hours_between_doses"
    t.text "notes"
    t.bigint "person_id", null: false
    t.date "start_date"
    t.datetime "updated_at", null: false
    t.index ["active"], name: "index_prescriptions_on_active"
    t.index ["dosage_id"], name: "index_prescriptions_on_dosage_id"
    t.index ["medicine_id"], name: "index_prescriptions_on_medicine_id"
    t.index ["person_id"], name: "index_prescriptions_on_person_id"
  end

  create_table "users", force: :cascade do |t|
    t.boolean "active", default: true, null: false
    t.datetime "created_at", null: false
    t.string "email_address", null: false
    t.string "password_digest"
    t.bigint "person_id", null: false
    t.integer "role", default: 4, null: false
    t.datetime "updated_at", null: false
    t.index ["email_address"], name: "index_users_on_email_address", unique: true
    t.index ["person_id"], name: "index_users_on_person_id", unique: true
  end

  create_table "versions", force: :cascade do |t|
    t.datetime "created_at"
    t.string "event", null: false
    t.string "ip"
    t.bigint "item_id", null: false
    t.string "item_type", null: false
    t.text "object"
    t.string "whodunnit"
    t.index ["created_at"], name: "index_versions_on_created_at"
    t.index ["event"], name: "index_versions_on_event"
    t.index ["item_type", "item_id"], name: "index_versions_on_item_type_and_item_id"
    t.index ["whodunnit"], name: "index_versions_on_whodunnit"
  end

  add_foreign_key "account_identities", "accounts", on_delete: :cascade
  add_foreign_key "account_login_change_keys", "accounts"
  add_foreign_key "account_otp_keys", "accounts", column: "id"
  add_foreign_key "account_password_reset_keys", "accounts"
  add_foreign_key "account_recovery_codes", "accounts", column: "id"
  add_foreign_key "account_remember_keys", "accounts"
  add_foreign_key "account_verification_keys", "accounts"
  add_foreign_key "carer_relationships", "people", column: "carer_id", deferrable: :deferred
  add_foreign_key "carer_relationships", "people", column: "patient_id", deferrable: :deferred
  add_foreign_key "dosages", "medicines", deferrable: :deferred
  add_foreign_key "medication_takes", "person_medicines", deferrable: :deferred
  add_foreign_key "medication_takes", "prescriptions", deferrable: :deferred
  add_foreign_key "people", "accounts", deferrable: :deferred
  add_foreign_key "person_medicines", "medicines", deferrable: :deferred
  add_foreign_key "person_medicines", "people", deferrable: :deferred
  add_foreign_key "prescriptions", "dosages", deferrable: :deferred
  add_foreign_key "prescriptions", "medicines", deferrable: :deferred
  add_foreign_key "prescriptions", "people", deferrable: :deferred
  add_foreign_key "users", "people", deferrable: :deferred
end
