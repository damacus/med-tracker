# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PortableData::Importer do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :schedules, :person_medications

  let(:household) { households(:fixture_household) }
  let(:source) { schedules(:john_paracetamol) }
  let(:membership) do
    household.household_memberships.find_or_create_by!(account: accounts(:admin)) do |record|
      record.role = :owner
      record.status = :active
      record.person = people(:john)
    end
  end
  let(:exporter) do
    PortableData::Exporter.new(
      household: household, membership: membership, passphrase: 'secret', person_ids: [source.person_id]
    )
  end
  let(:period) do
    MedicationPausePeriod.create!(schedule: source, reason: 'other', note: 'Original context',
                                  started_at: 2.days.ago, recorded_by_membership: membership)
  end

  def restore(rows, sources: [])
    payload = { format: 'medtracker.portable.v2', records: { medication_pause_periods: rows, schedules: sources } }
    PortableData::Importer.new(household: household, membership: membership,
                               envelope: PortableData::Encryptor.encrypt(payload, passphrase: 'secret'),
                               passphrase: 'secret', options: { dry_run: false }).call
  end

  def row
    {
      portable_id: SecureRandom.uuid, source_type: 'schedule', source_portable_id: source.portable_id,
      reason: 'other', note: 'Original context', started_at: 2.days.ago.iso8601(6), ended_at: nil,
      recorded_by_person_portable_id: membership.person.portable_id, resumed_by_person_portable_id: nil,
      legacy_context: false, created_at: 2.days.ago.iso8601(6)
    }
  end

  it 'exports v2 context and portable actor references without changing v1' do
    period
    exported = exporter.v2_payload.dig(:records, :medication_pause_periods).first

    expect(exported).to include(portable_id: period.portable_id, source_type: 'schedule',
                                source_portable_id: source.portable_id, reason: 'other', note: 'Original context',
                                recorded_by_person_portable_id: membership.person.portable_id, legacy_context: false)
    expect(exporter.payload.fetch(:records)).not_to have_key(:medication_pause_periods)
  end

  it 'restores an open interval once and reconciles source activity without changing stock' do
    exported = row
    stock = source.medication.current_supply

    expect([restore([exported]), restore([exported])]).to all(be_applied)
    expect(source.reload).not_to be_active
    expect(source.medication.reload.current_supply).to eq(stock)
    expect(source.medication_pause_periods.count).to eq(1)
    expect(source.medication_pause_periods.first.recorded_by_membership).to eq(membership)
  end

  it 'restores completed assignment history and makes its source active' do
    assignment = person_medications(:john_vitamin_d)
    assignment.update!(active: false)
    exported = row.merge(source_type: 'person_medication', source_portable_id: assignment.portable_id,
                         ended_at: 1.day.ago.iso8601(6), resumed_by_person_portable_id: membership.person.portable_id)

    expect(restore([exported])).to be_applied
    expect(assignment.reload).to be_active
    expect(assignment.medication_pause_periods.first.resumed_by_membership).to eq(membership)
  end

  it 'preserves unknown legacy starts and creation time' do
    exported = row.merge(reason: 'reason_not_recorded', legacy_context: true, started_at: nil,
                         recorded_by_person_portable_id: nil)

    expect(restore([exported])).to be_applied
    restored = source.medication_pause_periods.first
    expect(restored.started_at).to be_nil
    expect(restored.created_at.iso8601(6)).to eq(exported[:created_at])
  end

  it 'rejects two open intervals and rolls back all records' do
    result = restore([row, row])

    expect(result).not_to be_applied
    expect(source.medication_pause_periods).to be_empty
    expect(source.reload).to be_active
  end

  it 'rolls back a valid interval when a later interval has invalid times' do
    result = restore([row.merge(ended_at: 1.day.ago.iso8601), row.merge(ended_at: 4.days.ago.iso8601)])

    expect(result).not_to be_applied
    expect(source.medication_pause_periods).to be_empty
  end

  it 'rejects an unsupported source type' do
    expect(restore([row.merge(source_type: 'unknown')])).not_to be_applied
  end

  it 'retains unmappable actor references without attributing the pause to the importer' do
    exported = row.merge(recorded_by_person_portable_id: 'original-person')

    expect(restore([exported])).to be_applied
    restored = source.medication_pause_periods.first
    expect(restored.recorded_by_membership).to be_nil
    expect(restored).to be_imported_context
    expect(exporter.v2_payload.dig(:records, :medication_pause_periods).first)
      .to include(recorded_by_person_portable_id: 'original-person')
  end

  it 'round trips completed and open exported history without duplicating periods' do
    period.update!(ended_at: 1.day.ago, resumed_by_membership: membership)
    open_period = MedicationPausePeriod.create!(schedule: source, reason: 'clinician_advice',
                                                started_at: 1.hour.ago, recorded_by_membership: membership)
    rows = exporter.v2_payload.dig(:records, :medication_pause_periods)

    expect(restore(rows)).to be_applied
    expected_ids = [period.portable_id, open_period.portable_id]
    expect(source.medication_pause_periods.pluck(:portable_id)).to match_array(expected_ids)
    expect(source.reload).not_to be_active
  end

  it 'restores a v2 bundle into another household with portable source and unknown actor context' do
    period
    source.update!(active: false)
    source.person.update!(email: nil)
    destination = create(:household)
    owner = destination.household_memberships.create!(account: accounts(:admin), role: :owner, status: :active)
    result = described_class.new(household: destination, membership: owner,
                                 envelope: exporter.call(version: 2), passphrase: 'secret',
                                 options: { dry_run: false }).call

    expect(result).to be_applied
    restored = MedicationPausePeriod.find_by!(household: destination, portable_id: period.portable_id)
    expect(restored.schedule.portable_id).to eq(source.portable_id)
    expect(restored.schedule).not_to be_active
    expect(restored.recorded_by_membership).to be_nil
    expect(restored.imported_actor_references['recorded_by_person_portable_id']).to eq(membership.person.portable_id)
  end

  it 'rejects attempts to rewrite recorded context' do
    exported = row.merge(portable_id: period.portable_id, note: 'Changed history')

    expect(restore([exported])).not_to be_applied
    expect(period.reload.note).to eq('Original context')
  end

  it 'preserves native provenance and timestamps on identical reimport' do
    period
    source.update!(active: false)
    original = period.attributes
    exported = exporter.v2_payload.dig(:records, :medication_pause_periods).first

    expect(restore([exported])).to be_applied
    expect(period.reload.attributes).to eq(original)
  end

  it 'rejects closing native history through an imported resume claim' do
    period
    source.update!(active: false)
    exported = exporter.v2_payload.dig(:records, :medication_pause_periods).first
    exported[:ended_at] = Time.current.iso8601(6)
    exported[:resumed_by_person_portable_id] = membership.person.portable_id

    expect(restore([exported])).not_to be_applied
    expect(period.reload.ended_at).to be_nil
    expect(period).not_to be_imported_context
    expect(source.reload).not_to be_active
  end

  it 'allows imported open history to advance with preserved recording context' do
    exported = row
    expect(restore([exported])).to be_applied
    exported[:ended_at] = Time.current.iso8601(6)
    exported[:resumed_by_person_portable_id] = membership.person.portable_id

    expect(restore([exported])).to be_applied
    restored = source.medication_pause_periods.first
    expect(restored).to be_imported_context
    expect(restored.resumed_by_membership).to eq(membership)
    expect(source.reload).to be_active
  end

  it 'still requires known starts for imported nonlegacy records' do
    expect(restore([row.merge(started_at: nil)])).not_to be_applied
    expect(source.medication_pause_periods).to be_empty
  end

  it 'rejects attempts to replace an original recording actor' do
    period
    exported = exporter.v2_payload.dig(:records, :medication_pause_periods).first
    exported[:recorded_by_person_portable_id] = people(:jane).portable_id

    expect(restore([exported])).not_to be_applied
    expect(period.reload.recorded_by_membership).to eq(membership)
  end

  it 'rejects incomplete inactive source history instead of silently resuming it' do
    exported = exporter.v2_payload.dig(:records, :schedules).find { |row| row[:portable_id] == source.portable_id }
    exported[:active] = false

    expect(restore([], sources: [exported])).not_to be_applied
    expect(source.reload).to be_active
    expect(source.medication_pause_periods).to be_empty
  end

  it 'rejects foreign household sources' do
    foreign_household = create(:household)
    foreign = create(:schedule, household: foreign_household,
                                person: create(:person, household: foreign_household),
                                medication: create(:medication, household: foreign_household))

    expect(restore([row.merge(source_portable_id: foreign.portable_id)])).not_to be_applied
    expect(foreign.medication_pause_periods).to be_empty
  end
end
