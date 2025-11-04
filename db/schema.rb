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

ActiveRecord::Schema[8.1].define(version: 2025_11_03_230827) do
  create_table "carer_relationships", force: :cascade do |t|
    t.boolean "active", default: true, null: false
    t.integer "carer_id", null: false
    t.datetime "created_at", null: false
    t.integer "patient_id", null: false
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
    t.integer "medicine_id", null: false
    t.string "unit"
    t.datetime "updated_at", null: false
    t.index ["medicine_id"], name: "index_dosages_on_medicine_id"
  end

  create_table "medication_takes", force: :cascade do |t|
    t.decimal "amount_ml"
    t.datetime "created_at", null: false
    t.integer "person_medicine_id"
    t.integer "prescription_id"
    t.datetime "taken_at"
    t.datetime "updated_at", null: false
    t.index ["person_medicine_id"], name: "index_medication_takes_on_person_medicine_id"
    t.index ["prescription_id"], name: "index_medication_takes_on_prescription_id"
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

  create_table "passkeys_rails_agents", force: :cascade do |t|
    t.integer "authenticatable_id"
    t.string "authenticatable_type"
    t.datetime "created_at", null: false
    t.datetime "last_authenticated_at"
    t.datetime "registered_at"
    t.datetime "updated_at", null: false
    t.string "username", null: false
    t.string "webauthn_identifier"
    t.index ["authenticatable_type", "authenticatable_id"], name: "index_passkeys_rails_agents_on_authenticatable", unique: true
    t.index ["username"], name: "index_passkeys_rails_agents_on_username", unique: true
  end

  create_table "passkeys_rails_passkeys", force: :cascade do |t|
    t.integer "agent_id", null: false
    t.datetime "created_at", null: false
    t.string "identifier"
    t.string "public_key"
    t.integer "sign_count"
    t.datetime "updated_at", null: false
    t.index ["agent_id"], name: "index_passkeys_rails_passkeys_on_agent_id"
  end

  create_table "people", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.date "date_of_birth"
    t.string "email"
    t.boolean "has_capacity", default: true, null: false
    t.string "name", null: false
    t.integer "person_type", default: 0, null: false
    t.datetime "updated_at", null: false
    t.index ["email"], name: "index_people_on_email", unique: true
  end

  create_table "person_medicines", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "max_daily_doses"
    t.integer "medicine_id", null: false
    t.integer "min_hours_between_doses"
    t.text "notes"
    t.integer "person_id", null: false
    t.datetime "updated_at", null: false
    t.index ["medicine_id"], name: "index_person_medicines_on_medicine_id"
    t.index ["person_id", "medicine_id"], name: "index_person_medicines_on_person_id_and_medicine_id", unique: true
    t.index ["person_id"], name: "index_person_medicines_on_person_id"
  end

  create_table "prescriptions", force: :cascade do |t|
    t.boolean "active", default: true
    t.datetime "created_at", null: false
    t.integer "dosage_id", null: false
    t.integer "dose_cycle"
    t.date "end_date"
    t.string "frequency"
    t.integer "max_daily_doses", default: 4
    t.integer "medicine_id", null: false
    t.integer "min_hours_between_doses"
    t.text "notes"
    t.integer "person_id", null: false
    t.date "start_date"
    t.datetime "updated_at", null: false
    t.index ["dosage_id"], name: "index_prescriptions_on_dosage_id"
    t.index ["medicine_id"], name: "index_prescriptions_on_medicine_id"
    t.index ["person_id"], name: "index_prescriptions_on_person_id"
  end

  create_table "sessions", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "ip_address"
    t.datetime "updated_at", null: false
    t.string "user_agent"
    t.integer "user_id", null: false
    t.index ["user_id"], name: "index_sessions_on_user_id"
  end

  create_table "users", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "email_address", null: false
    t.string "password_digest", null: false
    t.integer "person_id", null: false
    t.integer "role", default: 4, null: false
    t.datetime "updated_at", null: false
    t.index ["email_address"], name: "index_users_on_email_address", unique: true
    t.index ["person_id"], name: "index_users_on_person_id", unique: true
  end

  add_foreign_key "carer_relationships", "people", column: "carer_id"
  add_foreign_key "carer_relationships", "people", column: "patient_id"
  add_foreign_key "dosages", "medicines"
  add_foreign_key "medication_takes", "person_medicines"
  add_foreign_key "medication_takes", "prescriptions"
  add_foreign_key "passkeys_rails_passkeys", "passkeys_rails_agents", column: "agent_id"
  add_foreign_key "person_medicines", "medicines"
  add_foreign_key "person_medicines", "people"
  add_foreign_key "prescriptions", "dosages"
  add_foreign_key "prescriptions", "medicines"
  add_foreign_key "prescriptions", "people"
  add_foreign_key "sessions", "users"
  add_foreign_key "users", "people"
end
