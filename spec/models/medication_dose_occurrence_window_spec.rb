require 'rails_helper'

RSpec.describe MedicationDoseOccurrence do
  let(:source) { create(:person_medication, :routine, dose_cycle: :monthly) }
  let(:date) { Date.current.beginning_of_month }

  def saved_outcome
    described_class.create!(person_medication: source, window_starts_on: date, position: 1)
  end

  it 'snapshots the inclusive end of a routine cycle' do
    expect(saved_outcome.window_ends_on).to eq(date.end_of_month)
  end

  it 'preserves the stored window after the source cycle changes' do
    record = saved_outcome
    source.update!(dose_cycle: :daily)
    row = MedicationAdministration::OccurrenceProjection.new(
      source: source, start_date: date, end_date: date
    ).call.first
    expect(record.reload.window_ends_on).to eq(date.end_of_month)
    expect(row.window_ends_on).to eq(date.end_of_month)
  end

  it 'uses one day for a formal schedule' do
    schedule = create(:schedule)
    record = described_class.create!(schedule: schedule, window_starts_on: date, position: 1)
    expect(record.window_ends_on).to eq(date)
  end

  it 'rejects a reversed window' do
    record = described_class.new(person_medication: source, window_starts_on: date,
                                 window_ends_on: date - 1, position: 1)
    expect(record).not_to be_valid
    expect(record.errors[:window_ends_on]).to be_present
  end

  it 'does not allow changing an existing window boundary' do
    record = saved_outcome
    expect(record.update(window_ends_on: date + 1)).to be(false)
    expect(record.reload.window_ends_on).to eq(date.end_of_month)
  end
end
