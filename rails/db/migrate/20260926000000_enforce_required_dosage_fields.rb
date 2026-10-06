class EnforceRequiredDosageFields < ActiveRecord::Migration[8.1]
  REQUIRED_FIELDS = %i[
    amount unit frequency default_max_daily_doses default_min_hours_between_doses default_dose_cycle
  ].freeze

  def up
    REQUIRED_FIELDS.each do |field|
      change_column_null :dosages, field, false
    rescue ActiveRecord::NotNullViolation
      raise ActiveRecord::MigrationError,
            "Cannot require dosages.#{field}: existing NULL values must be repaired before migration"
    end
  end

  def down
    REQUIRED_FIELDS.each { |field| change_column_null :dosages, field, true }
  end
end
