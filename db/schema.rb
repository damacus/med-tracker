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

ActiveRecord::Schema[8.0].define(version: 2025_09_29_163001) do
  create_table "dosages", force: :cascade do |t|
    t.integer "medicine_id", null: false
    t.decimal "amount"
    t.string "unit"
    t.string "frequency"
    t.string "description"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["medicine_id"], name: "index_dosages_on_medicine_id"
  end

  create_table "medication_takes", force: :cascade do |t|
    t.integer "prescription_id", null: false
    t.datetime "taken_at"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["prescription_id"], name: "index_medication_takes_on_prescription_id"
  end

  create_table "medicines", force: :cascade do |t|
    t.string "name"
    t.integer "current_supply"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.float "dosage_amount"
    t.string "dosage_unit"
    t.integer "stock"
    t.date "expiry_date"
    t.text "description"
    t.text "warnings"
    t.integer "reorder_threshold", default: 10, null: false
  end

  create_table "people", force: :cascade do |t|
    t.string "name", null: false
    t.string "email"
    t.date "date_of_birth"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["email"], name: "index_people_on_email", unique: true
  end

  create_table "prescriptions", force: :cascade do |t|
    t.integer "medicine_id", null: false
    t.integer "dosage_id", null: false
    t.date "start_date"
    t.date "end_date"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.string "frequency"
    t.text "notes"
    t.integer "max_daily_doses", default: 4
    t.integer "min_hours_between_doses"
    t.integer "dose_cycle"
    t.boolean "active", default: true
    t.integer "person_id", null: false
    t.index ["dosage_id"], name: "index_prescriptions_on_dosage_id"
    t.index ["medicine_id"], name: "index_prescriptions_on_medicine_id"
    t.index ["person_id"], name: "index_prescriptions_on_person_id"
  end

  create_table "sessions", force: :cascade do |t|
    t.integer "user_id", null: false
    t.string "ip_address"
    t.string "user_agent"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["user_id"], name: "index_sessions_on_user_id"
  end

  create_table "users", force: :cascade do |t|
    t.string "email_address", null: false
    t.string "password_digest", null: false
    t.integer "role", default: 0, null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.integer "person_id", null: false
    t.index ["email_address"], name: "index_users_on_email_address", unique: true
    t.index ["person_id"], name: "index_users_on_person_id", unique: true
  end

  add_foreign_key "dosages", "medicines"
  add_foreign_key "medication_takes", "prescriptions"
  add_foreign_key "prescriptions", "dosages"
  add_foreign_key "prescriptions", "medicines"
  add_foreign_key "prescriptions", "people"
  add_foreign_key "sessions", "users"
  add_foreign_key "users", "people"
end
