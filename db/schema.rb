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

ActiveRecord::Schema[8.0].define(version: 2025_06_23_093841) do
  create_table "dosage_options", force: :cascade do |t|
    t.integer "medicine_id", null: false
    t.decimal "amount", precision: 10, scale: 2, null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["medicine_id", "amount"], name: "index_dosage_options_on_medicine_id_and_amount", unique: true
    t.index ["medicine_id"], name: "index_dosage_options_on_medicine_id"
  end

  create_table "medication_takes", force: :cascade do |t|
    t.integer "prescription_id", null: false
    t.datetime "taken_at"
    t.text "notes"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.decimal "amount_ml"
    t.index ["prescription_id"], name: "index_medication_takes_on_prescription_id"
  end

  create_table "medications", force: :cascade do |t|
    t.string "name"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
  end

  create_table "medicine_dosages", force: :cascade do |t|
    t.decimal "amount"
    t.string "unit"
    t.string "description"
    t.boolean "is_default"
    t.integer "medicine_id", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["medicine_id"], name: "index_medicine_dosages_on_medicine_id"
  end

  create_table "medicines", force: :cascade do |t|
    t.string "name"
    t.text "description"
    t.decimal "dosage", precision: 10, scale: 2
    t.text "warnings"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.string "unit", default: "tablet", null: false
    t.decimal "min_dosage", precision: 10, scale: 2
    t.decimal "max_dosage", precision: 10, scale: 2
  end

  create_table "people", force: :cascade do |t|
    t.string "name"
    t.date "date_of_birth"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.string "email"
    t.index ["email"], name: "index_people_on_email"
  end

  create_table "prescriptions", force: :cascade do |t|
    t.integer "person_id", null: false
    t.integer "medicine_id", null: false
    t.string "dosage"
    t.string "frequency"
    t.date "start_date"
    t.date "end_date"
    t.text "notes"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.integer "min_hours_between_doses"
    t.integer "max_daily_doses"
    t.string "dose_cycle"
    t.index ["medicine_id"], name: "index_prescriptions_on_medicine_id"
    t.index ["person_id"], name: "index_prescriptions_on_person_id"
  end

  create_table "recommended_dosages", force: :cascade do |t|
    t.integer "medicine_id", null: false
    t.integer "min_age"
    t.integer "max_age"
    t.decimal "amount_ml"
    t.integer "frequency_per_day"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["medicine_id"], name: "index_recommended_dosages_on_medicine_id"
  end

  add_foreign_key "dosage_options", "medicines"
  add_foreign_key "medication_takes", "prescriptions"
  add_foreign_key "medicine_dosages", "medicines"
  add_foreign_key "prescriptions", "medicines"
  add_foreign_key "prescriptions", "people"
  add_foreign_key "recommended_dosages", "medicines"
end
