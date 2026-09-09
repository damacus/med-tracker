require 'rails_helper'

RSpec.describe PortableData::Importer do
  let(:destination) { create(:household) }
  let(:owner) { owner_for(destination) }
  let(:passphrase) { 'portable outcome test phrase' }
  let(:source) do
    household = create(:household)
    create(:schedule, household: household, person: create(:person, household: household),
                      medication: create(:medication, household: household, current_supply: 100),
                      frequency: 'Daily', max_daily_doses: 2)
  end

  def owner_for(household)
    account = Account.create!(email: "outcome-import-#{SecureRandom.hex(8)}@example.test", status: :verified)
    household.household_memberships.create!(account: account, role: :owner, status: :active)
  end

  def exported_payload
    actor = owner_for(source.household)
    create_outcomes(actor)
    PortableData::Exporter.new(household: source.household, membership: actor, passphrase: passphrase)
                          .payload(format: 'medtracker.portable.v2').deep_stringify_keys
  end

  def create_outcomes(actor)
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current, position: 1, outcome: 'not_taken', reason: 'unwell', note: 'Resting',
      resolved_at: Time.current, resolved_by_membership: actor
    )
    take = create(:medication_take, :for_schedule, household: source.household, schedule: source,
                                                   taken_at: Time.current)
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.current, position: 2, outcome: 'taken', medication_take: take,
      resolved_at: Time.current, resolved_by_membership: actor
    )
  end

  def restore(payload, dry_run: false, membership: owner)
    described_class.new(household: destination, membership: membership,
                        envelope: PortableData::Encryptor.encrypt(payload, passphrase: passphrase),
                        passphrase: passphrase, options: { dry_run: dry_run }).call
  end

  it 'round-trips outcomes and links with local import attribution' do
    payload = exported_payload
    result = restore(payload)
    expect(result).to be_applied
    restored = MedicationDoseOccurrence.where(household: destination).order(:position)
    expect(restored.map(&:portable_id)).to eq(payload.dig('records', 'dose_occurrences').pluck('portable_id'))
    expect(restored.first).to have_attributes(outcome: 'not_taken', reason: 'unwell', note: 'Resting')
    expect(restored.last.medication_take.portable_id).to eq(payload.dig('records',
                                                                        'medication_takes').sole['portable_id'])
    expect(restored.map(&:resolved_by_membership)).to all(eq(owner))
  end

  it 'preserves stock and safely replays an unchanged import' do
    payload = exported_payload
    expect(restore(payload)).to be_applied
    expect(destination.medications.sole.current_supply).to eq(source.medication.reload.current_supply)
    expect { expect(restore(payload)).to be_applied }.not_to change(MedicationDoseOccurrence, :count)
  end

  it 'rejects a missing reference in dry run without exposing the supplied identifier' do
    payload = exported_payload
    missing_id = SecureRandom.uuid
    payload.dig('records', 'dose_occurrences').first['source_portable_id'] = missing_id
    result = restore(payload, dry_run: true)
    expect(result).not_to be_applied
    expect(result.errors.join).to include('dose_occurrences', 'source_portable_id')
    expect(result.errors.join).not_to include(missing_id)
    expect(destination.people).to be_empty
  end

  it 'does not resolve a source reference from another household' do
    payload = exported_payload
    payload['records']['schedules'] = []
    result = restore(payload, dry_run: true)
    expect(result).not_to be_applied
    expect(result.errors).not_to be_empty
    expect(destination.people).to be_empty
  end

  it 'rolls back the graph when an outcome is invalid' do
    payload = exported_payload
    payload.dig('records', 'dose_occurrences').last['outcome'] = 'invalid'
    [true, false].each do |dry_run|
      result = restore(payload, dry_run: dry_run)
      expect(result).not_to be_applied
      expect(result.errors).not_to be_empty
    end
    expect(destination.people).to be_empty
    expect(MedicationTake.where(household: destination)).to be_empty
  end

  it 'rejects an import that would overwrite a later local decision' do
    payload = exported_payload
    expect(restore(payload)).to be_applied
    record = MedicationDoseOccurrence.find_by!(household: destination, outcome: 'not_taken')
    record.update!(reason: 'refused')
    result = restore(payload, dry_run: true)
    expect(result).not_to be_applied
    expect(result.errors).not_to be_empty
    expect(record.reload.reason).to eq('refused')
  end

  it 'rolls back restored outcomes and inventory when a later health event fails validation' do
    payload = exported_payload
    payload['records']['health_events'] = [{
      'portable_id' => SecureRandom.uuid, 'person_portable_id' => source.person.portable_id,
      'title' => '', 'event_kind' => 'suspected_side_effect', 'started_on' => Date.current.iso8601
    }]
    expect(restore(payload)).not_to be_applied
    expect(destination.people).to be_empty
    expect(destination.medications).to be_empty
    expect(MedicationDoseOccurrence.where(household: destination)).to be_empty
    expect(MedicationTake.where(household: destination)).to be_empty
  end

  it 'does not silently accept outcome rows inside the legacy v1 format' do
    payload = exported_payload.merge('format' => 'medtracker.portable.v1')
    expect { restore(payload) }.to raise_error(described_class::Error, /Unsupported portable record types/)
  end

  it 'rejects duplicate occurrence identities before restoring any records' do
    payload = exported_payload
    duplicate = payload.dig('records', 'dose_occurrences').first.merge('portable_id' => SecureRandom.uuid)
    payload['records']['dose_occurrences'] << duplicate
    result = restore(payload, dry_run: true)
    expect(result.errors.join).to include('duplicate occurrence')
    expect(destination.people).to be_empty
  end

  it 'rejects reuse of one take across incoming occurrence positions in dry run and apply' do
    payload = exported_payload
    duplicate = payload.dig('records', 'dose_occurrences').last.merge('portable_id' => SecureRandom.uuid,
                                                                      'position' => 3)
    payload['records']['dose_occurrences'] << duplicate

    [true, false].each do |dry_run|
      result = restore(payload, dry_run: dry_run)
      expect(result.errors.join).to include('take is already linked')
      expect(result).not_to be_applied
    end
    expect(destination.people).to be_empty
  end

  it 'rejects linking an existing take to a different incoming occurrence' do
    payload = exported_payload
    expect(restore(payload)).to be_applied
    duplicate = payload.dig('records', 'dose_occurrences').last.merge('portable_id' => SecureRandom.uuid,
                                                                      'position' => 3)
    payload['records'] = { 'dose_occurrences' => [duplicate] }

    [true, false].each do |dry_run|
      result = restore(payload, dry_run: dry_run)
      expect(result.errors.join).to include('take is already linked')
      expect(result).not_to be_applied
    end
    expect(MedicationDoseOccurrence.where(household: destination).count).to eq(2)
  end

  it 'rejects a take linked to a different occurrence window' do
    payload = exported_payload
    payload.dig('records', 'dose_occurrences').last['window_starts_on'] = Date.yesterday.iso8601
    result = restore(payload, dry_run: true)
    expect(result.errors.join).to include('take does not match')
    expect(destination.people).to be_empty
  end

  it 'checks the stored timestamp when an imported take already exists' do
    payload = exported_payload
    legacy = payload.deep_dup
    legacy['format'] = 'medtracker.portable.v1'
    legacy['records'].delete('dose_occurrences')
    expect(restore(legacy)).to be_applied
    row = payload.dig('records', 'dose_occurrences').last
    row['window_starts_on'] = Date.yesterday.iso8601
    take = payload.dig('records', 'medication_takes').sole
    take['taken_at'] = "#{Date.yesterday.iso8601}T12:00:00Z"
    payload['records'] = { 'dose_occurrences' => [row], 'medication_takes' => [take] }

    result = restore(payload, dry_run: true)
    expect(result.errors.join).to include('take does not match')
    expect(MedicationDoseOccurrence.where(household: destination)).to be_empty
  end

  it 'rejects a new portable ID for an existing occurrence identity' do
    payload = exported_payload
    expect(restore(payload)).to be_applied
    payload.dig('records', 'dose_occurrences').first['portable_id'] = SecureRandom.uuid
    expect(restore(payload, dry_run: true).errors.join).to include('existing history')
  end

  it 'restores health events carried by the existing v2 mobile payload' do
    exported_payload
    event = HealthEvent.create!(person: source.person, event_kind: :suspected_side_effect,
                                title: 'Nausea', started_on: Date.current)
    event.medications = [source.medication]
    exporter = PortableData::Exporter.new(household: source.household, membership: owner_for(source.household),
                                          passphrase: passphrase)
    expect(restore(exporter.mobile_payload(format: 'medtracker.portable.v2').deep_stringify_keys)).to be_applied
    restored = HealthEvent.find_by!(household: destination, portable_id: event.portable_id)
    expect(restored.title).to eq('Nausea')
    expect(restored.medications.pluck(:portable_id)).to eq([source.medication.portable_id])
  end

  it 'requires a current manage grant for delegated outcome-only imports' do
    payload = exported_payload
    expect(restore(payload)).to be_applied
    payload['records'] = { 'dose_occurrences' => payload.dig('records', 'dose_occurrences') }
    delegate = owner_for(destination)
    delegate.update!(role: :member)
    grant = PersonAccessGrant.create!(household: destination, household_membership: delegate,
                                      person: destination.people.sole, access_level: :manage,
                                      relationship_type: :family_member, expires_at: 1.hour.from_now)
    expect(restore(payload, dry_run: true, membership: delegate).errors).to be_empty
    grant.update!(expires_at: 1.minute.ago)
    expect { restore(payload, dry_run: true, membership: delegate) }.to raise_error(Pundit::NotAuthorizedError)
  end
end
