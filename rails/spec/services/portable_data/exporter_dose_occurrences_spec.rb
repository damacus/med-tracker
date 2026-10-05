require 'rails_helper'

RSpec.describe PortableData::Exporter do
  let(:household) { create(:household) }
  let(:membership) do
    account = Account.create!(email: 'outcome-export@example.test', status: :verified)
    household.household_memberships.create!(account: account, role: :owner, status: :active)
  end
  let(:source) do
    create(:schedule, household: household, person: create(:person, household: household),
                      medication: create(:medication, household: household), frequency: 'Daily')
  end
  let(:exporter) do
    described_class.new(household: household, membership: membership, passphrase: 'portable test phrase')
  end

  def record_outcome(**attributes)
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current, position: 1, outcome: 'not_taken', reason: 'unwell', note: 'Resting',
      resolved_at: Time.current, resolved_by_membership: membership, **attributes
    )
  end

  it 'exports stored not-taken context with portable source references in v2' do
    record = record_outcome
    payload = exporter.payload(format: 'medtracker.portable.v2')
    row = payload.fetch(:records).fetch(:dose_occurrences).sole

    expect(payload.fetch(:format)).to eq('medtracker.portable.v2')
    expect(row).to include(portable_id: record.portable_id, source_type: 'schedule',
                           source_portable_id: source.portable_id, window_starts_on: Date.current.iso8601,
                           outcome: 'not_taken', reason: 'unwell', note: 'Resting', medication_take_portable_id: nil)
    expect(row.keys).not_to include(:id, :household_id, :schedule_id, :resolved_by_membership_id)
  end

  it 'exports taken linkage without touching inventory and retains retired sources' do
    take = create(:medication_take, :for_schedule, household: household, schedule: source, taken_at: Time.current)
    record_outcome(outcome: 'taken', reason: nil, note: nil, medication_take: take)
    source.retire!
    supply = source.medication.current_supply
    payload = exporter.payload(format: 'medtracker.portable.v2')

    expect(payload.dig(:records, :dose_occurrences).sole.fetch(:medication_take_portable_id)).to eq(take.portable_id)
    expect(payload.dig(:records, :schedules).pluck(:portable_id)).to include(source.portable_id)
    expect(source.medication.reload.current_supply).to eq(supply)
  end

  it 'preserves the v1 collection shape and exports no derived open rows' do
    source
    expect(exporter.payload.fetch(:records)).not_to have_key(:dose_occurrences)
    expect(exporter.payload(format: 'medtracker.portable.v2').dig(:records, :dose_occurrences)).to be_empty
    expect(MedicationDoseOccurrence.count).to eq(0)
  end

  it 'encrypts a requested v2 bundle and rejects unsupported versions' do
    record_outcome
    payload = PortableData::Encryptor.decrypt(exporter.call(format: 'medtracker.portable.v2'),
                                              passphrase: 'portable test phrase')
    expect(payload.fetch('format')).to eq('medtracker.portable.v2')
    expect(payload.dig('records', 'dose_occurrences').sole.fetch('note')).to eq('Resting')
    expect do
      exporter.payload(format: 'unknown')
    end.to raise_error(described_class::Error, 'Unsupported portable data format')
  end

  it 'limits outcome records to the selected exported people' do
    record_outcome
    scoped = described_class.new(household: household, membership: membership, passphrase: 'portable test phrase',
                                 person_ids: [create(:person, household: household).id])
    expect(scoped.payload(format: 'medtracker.portable.v2').dig(:records, :dose_occurrences)).to be_empty
  end
end
