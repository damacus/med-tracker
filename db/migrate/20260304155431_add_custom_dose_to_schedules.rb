class AddCustomDoseToSchedules < ActiveRecord::Migration[8.1]
  def change
    add_column :schedules, :custom_dose_amount, :decimal, precision: 10, scale: 2
    add_column :schedules, :custom_dose_unit, :string
    change_column_null :schedules, :dosage_id, true
  end
end
