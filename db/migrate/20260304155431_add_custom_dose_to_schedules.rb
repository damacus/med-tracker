class AddCustomDoseToSchedules < ActiveRecord::Migration[8.1]
  def change
    add_column :schedules, :custom_dose_amount, :decimal, precision: 10, scale: 2
    add_column :schedules, :custom_dose_unit, :string
    add_column :schedules, :schedule_type, :string, default: 'scheduled'
    change_column_null :schedules, :dosage_id, true
  end
end
