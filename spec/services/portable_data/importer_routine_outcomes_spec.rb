require 'rails_helper'

RSpec.describe PortableData::Importer do
  let(:source_household) { create(:household) }
  let(:destination) { create(:household) }
  let(:source) do
    create(:person_medication, :routine, person: create(:person, household: source_household),
                                         dose_cycle: :monthly, max_daily_doses: 2, created_at: 2.months.ago)
  end

  before { travel_to(Time.zone.local(2026, 9, 9, 12)) }

  def owner_for(household)
    account = Account.create!(email: "routine-import-#{SecureRandom.hex(8)}@example.test", status: :verified)
    household.household_memberships.create!(account: account, role: :owner, status: :active)
  end

  def payload
    actor = owner_for(source_household)
    create_outcomes(actor)
    PortableData::Exporter.new(household: source_household, membership: actor, passphrase: 'routine test phrase')
                          .payload(format: 'medtracker.portable.v2').deep_stringify_keys
  end

  def create_outcomes(actor)
    take = create(:medication_take, person_medication: source, taken_at: Time.current)
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current.beginning_of_month, position: 1, outcome: 'not_taken',
      reason: 'unwell', resolved_at: Time.current, resolved_by_membership: actor
    )
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current.beginning_of_month, position: 2, outcome: 'taken',
      medication_take: take, resolved_at: Time.current, resolved_by_membership: actor
    )
  end

  def restore(data, dry_run: false)
    actor = destination.household_memberships.first || owner_for(destination)
    envelope = PortableData::Encryptor.encrypt(data, passphrase: 'routine test phrase')
    described_class.new(household: destination, membership: actor, envelope: envelope,
                        passphrase: 'routine test phrase', options: { dry_run: dry_run }).call
  end

  it 'round-trips monthly outcomes, cycle boundaries and take linkage' do
    data = payload
    expect(data.dig('records', 'dose_occurrences').pluck('window_ends_on')).to eq(%w[2026-09-30 2026-09-30])
    expect(restore(data)).to be_applied
    rows = MedicationDoseOccurrence.where(household: destination).order(:position)
    expect(rows.pluck(:window_ends_on)).to eq([Date.new(2026, 9, 30)] * 2)
    expect(rows.last.medication_take.taken_at.to_date).to eq(Date.new(2026, 9, 9))
  end

  it 'preserves stock and replays an unchanged routine import' do
    data = payload
    expect(restore(data)).to be_applied
    expect(destination.medications.sole.current_supply).to eq(source.medication.reload.current_supply)
    expect { expect(restore(data)).to be_applied }.not_to change(MedicationDoseOccurrence, :count)
  end

  it 'preserves an exported monthly boundary after the assignment changes to daily' do
    data = payload
    data.dig('records', 'person_medications').sole['dose_cycle'] = 'daily'

    expect(restore(data)).to be_applied
    expect(MedicationDoseOccurrence.where(household: destination).pluck(:window_ends_on))
      .to eq([Date.new(2026, 9, 30)] * 2)
  end

  it 'accepts older v2 rows without a cycle-end field and can replay them' do
    data = payload
    data.dig('records', 'dose_occurrences').each { |row| row.delete('window_ends_on') }

    expect(restore(data)).to be_applied
    expect(restore(data)).to be_applied
  end

  it 'rejects an import that changes a saved cycle boundary' do
    data = payload
    expect(restore(data)).to be_applied
    data.dig('records', 'dose_occurrences').first['window_ends_on'] = '2026-09-01'

    expect(restore(data, dry_run: true).errors.join).to include('conflicts with existing history')
  end

  it 'rejects as-needed assignment references during dry run' do
    data = payload
    data['records']['dose_occurrences'].select! { |row| row['outcome'] == 'not_taken' }
    data['records']['medication_takes'] = []
    data.dig('records', 'person_medications').sole['administration_kind'] = 'as_needed'

    expect(restore(data, dry_run: true).errors).not_to be_empty
    expect(destination.people).to be_empty
  end

  it 'rejects a linked administration outside the cycle without restoring any records' do
    data = payload
    data.dig('records', 'medication_takes').sole['taken_at'] = Time.zone.local(2026, 8, 31, 12).iso8601

    expect(restore(data).errors).not_to be_empty
    expect(destination.people).to be_empty
  end

  it 'rejects a reversed saved window in dry run' do
    data = payload
    data['records']['dose_occurrences'].select! { |row| row['outcome'] == 'not_taken' }
    data['records']['medication_takes'] = []
    data.dig('records', 'dose_occurrences').first['window_ends_on'] = '2026-08-31'

    expect(restore(data, dry_run: true).errors).not_to be_empty
  end
end
