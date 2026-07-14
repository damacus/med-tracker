# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationAdministration::RestoreHistory do
  subject(:restore) do
    described_class.new(household: household, rows: [row], conflict_error: error_class)
  end

  let(:error_class) { Class.new(StandardError) }
  let(:household) { create(:household) }
  let(:medication) { create(:medication, household: household, location: create(:location, household: household)) }
  let(:schedule) do
    create(:schedule, household: household, person: create(:person, household: household), medication: medication)
  end
  let(:row) do
    {
      portable_id: 'restored-take',
      client_uuid: 'restored-take-client',
      source_type: 'schedule',
      source_portable_id: schedule.portable_id,
      taken_at: '2026-02-01T08:30:00Z',
      dose_amount: 5,
      dose_unit: 'mg',
      taken_from_medication_portable_id: medication.portable_id,
      taken_from_location_portable_id: medication.location.portable_id
    }.with_indifferent_access
  end

  it 'restores immutable dose history without mutating stock' do
    original_supply = medication.current_supply

    expect { restore.call }.to change(MedicationTake, :count).by(1)
    expect(medication.reload.current_supply).to eq(original_supply)

    expect(MedicationTake.find_by!(portable_id: 'restored-take')).to have_attributes(
      schedule: schedule,
      dose_amount: 5,
      dose_unit: 'mg',
      taken_from_medication: medication,
      taken_from_location: medication.location
    )
  end

  it 'is idempotent for identical history' do
    2.times { restore.call }

    expect(MedicationTake.where(household: household, portable_id: 'restored-take').count).to eq(1)
  end

  it 'rejects conflicting immutable history' do
    restore.call
    conflicting_row = row.merge(dose_amount: 7)

    expect do
      described_class.new(household: household, rows: [conflicting_row], conflict_error: error_class).call
    end.to raise_error(error_class, /immutable medication take restored-take conflicts/)
  end

  it 'rejects unsupported source types with the importer error contract' do
    unsupported_row = row.merge(source_type: 'prescription')

    expect do
      described_class.new(household: household, rows: [unsupported_row], conflict_error: error_class).call
    end.to raise_error(error_class, 'Unsupported medication take source type')
  end
end
