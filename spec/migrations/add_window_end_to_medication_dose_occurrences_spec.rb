require 'rails_helper'

load Rails.root.join('db/migrate/20260909020000_add_window_end_to_medication_dose_occurrences.rb') unless
  defined?(AddWindowEndToMedicationDoseOccurrences)

RSpec.describe AddWindowEndToMedicationDoseOccurrences do
  delegate :connection, to: :'ActiveRecord::Base'

  it 'backfills both households through forced tenant isolation and restores the prior context' do
    records = [routine_outcome(:weekly), routine_outcome(:monthly)]
    records.each { |record| record.update_column(:window_ends_on, nil) }
    previous = connection.select_value("SELECT current_setting('med_tracker.current_household_id', true)")
    with_owner_role { described_class.new.send(:backfill_windows) }

    expect(records.first.reload.window_ends_on).to eq(records.first.window_starts_on + 6)
    expect(records.last.reload.window_ends_on).to eq(records.last.window_starts_on.end_of_month)
    restored = connection.select_value("SELECT current_setting('med_tracker.current_household_id', true)")
    expect(restored.presence).to eq(previous.presence)
  end

  def routine_outcome(cycle)
    household = create(:household)
    person = create(:person, household: household)
    medication = create(:medication, household: household)
    source = create(:person_medication, :routine, person: person, medication: medication, dose_cycle: cycle)
    date = DoseCycle.new(cycle).range_for(Time.current).begin.to_date
    MedicationDoseOccurrence.create!(person_medication: source, window_starts_on: date, position: 1)
  end

  def with_owner_role
    connection.execute('SET LOCAL ROLE med_tracker_owner')
    yield
  ensure
    connection.execute('RESET ROLE')
  end
end
