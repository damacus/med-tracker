class AddTimingRestrictionsToPrescriptions < ActiveRecord::Migration[8.0]
  def change
    add_column :prescriptions, :min_hours_between_doses, :integer
    add_column :prescriptions, :max_daily_doses, :integer
    add_column :prescriptions, :dose_cycle, :string
  end
end
