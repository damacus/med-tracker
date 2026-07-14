# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PortableData::Exporter do
  let(:passphrase) { 'correct horse battery staple' }

  def account(email)
    Account.create!(email: email, status: :verified)
  end

  def membership(household:, role:, email:)
    household.household_memberships.create!(account: account(email), role: role, status: :active)
  end

  def grant_manage_access(membership, person)
    PersonAccessGrant.create!(
      household: person.household,
      household_membership: membership,
      person: person,
      access_level: :manage,
      relationship_type: :self,
      granted_by_membership: membership
    )
  end

  def create_portable_person_graph(household:, name:, medication_name:)
    location = create(:location, household: household, name: "#{name} Cabinet")
    person = create(:person, household: household, name: name)
    person.location_memberships.create!(household: household, location: location)
    medication = create(:medication, household: household, location: location, name: medication_name)
    dosage = create(:dosage, household: household, medication: medication)
    schedule = create(:schedule, household: household, person: person, medication: medication, dosage: dosage)
    create(:person_medication, household: household, person: person, medication: medication, dosage: dosage)
    create(:medication_take, :for_schedule, household: household, schedule: schedule, client_uuid: SecureRandom.uuid)
    create(:notification_preference, household: household, person: person)
    person
  end

  def export_envelope(household:, membership:)
    described_class.new(household: household, membership: membership, passphrase: passphrase).call
  end

  def export_payload(household:, membership:)
    PortableData::Encryptor.decrypt(export_envelope(household: household, membership: membership),
                                    passphrase: passphrase)
  end

  def create_health_event_medication(household:)
    person = create(:person, household: household)
    medication = create(:medication, household: household, name: 'Health Event Medicine')
    event = HealthEvent.create!(
      person: person,
      event_kind: :suspected_side_effect,
      title: 'Nausea',
      started_on: Date.new(2026, 7, 13)
    )
    HealthEventMedication.create!(health_event: event, medication: medication)
    [event, medication]
  end

  it 'exports an encrypted bundle for every person an owner can manage' do
    household = create(:household)
    owner = membership(household: household, role: :owner, email: 'owner@example.test')
    person = create_portable_person_graph(household: household, name: 'Owner Patient',
                                          medication_name: 'Quiet Medicine')

    envelope = export_envelope(household: household, membership: owner)
    payload = PortableData::Encryptor.decrypt(envelope, passphrase: passphrase)

    expect([envelope.fetch(:format), payload.fetch('format'), payload.fetch('scope')]).to eq(
      ['medtracker.portable.encrypted.v1', 'medtracker.portable.v1', 'single_person']
    )
    expect(envelope.to_json).not_to include('Quiet Medicine')
    expect(payload.dig('records', 'people').pluck('portable_id')).to contain_exactly(person.portable_id)
    expect(payload.dig('records', 'medications').pluck('name')).to contain_exactly('Quiet Medicine')
    expect(payload.dig('records', 'medications').pluck('location_portable_id')).to all(be_present)
  end

  it 'exports unassigned household records only in the household-wide bundle' do
    household = create(:household)
    owner = membership(household: household, role: :owner, email: 'household-export-owner@example.test')
    create_portable_person_graph(household: household, name: 'Assigned Patient',
                                 medication_name: 'Assigned Medicine')
    unassigned = unassigned_records(household)
    exporter = described_class.new(household: household, membership: owner, passphrase: passphrase)
    household_payload = exporter.household_payload

    expect(household_payload.fetch(:scope)).to eq('household')
    verify_household_records(household_payload.fetch(:records), unassigned)
    verify_single_person_records(exporter.payload.fetch(:records), unassigned)
  end

  def unassigned_records(household)
    standalone_location = create(:location, household: household, name: 'Emergency Store')
    medication_location = create(:location, household: household, name: 'Unassigned Medicine Store')
    medication = create(
      :medication,
      household: household,
      location: medication_location,
      name: 'Unassigned Medicine'
    )
    dosage = create(:dosage, household: household, medication: medication)
    { locations: [standalone_location, medication_location], medication: medication, dosage: dosage }
  end

  def verify_household_records(records, unassigned)
    verify_household_locations(records, unassigned)
    verify_household_medication(records, unassigned)
    verify_household_dosage(records, unassigned)
  end

  def verify_household_locations(records, unassigned)
    expect(records.fetch(:locations).pluck(:portable_id))
      .to include(*unassigned.fetch(:locations).map(&:portable_id))
  end

  def verify_household_medication(records, unassigned)
    expect(records.fetch(:medications).pluck(:portable_id)).to include(unassigned.fetch(:medication).portable_id)
  end

  def verify_household_dosage(records, unassigned)
    expect(records.fetch(:dosage_options).pluck(:portable_id)).to include(unassigned.fetch(:dosage).portable_id)
  end

  def verify_single_person_records(records, unassigned)
    verify_single_person_locations(records, unassigned)
    verify_single_person_medication(records, unassigned)
    verify_single_person_dosage(records, unassigned)
  end

  def verify_single_person_locations(records, unassigned)
    expect(records.fetch(:locations).pluck(:portable_id))
      .not_to include(*unassigned.fetch(:locations).map(&:portable_id))
  end

  def verify_single_person_medication(records, unassigned)
    expect(records.fetch(:medications).pluck(:portable_id)).not_to include(unassigned.fetch(:medication).portable_id)
  end

  def verify_single_person_dosage(records, unassigned)
    expect(records.fetch(:dosage_options).pluck(:portable_id)).not_to include(unassigned.fetch(:dosage).portable_id)
  end

  it 'excludes medications referenced only by health events from encrypted bundles' do
    household = create(:household)
    owner = membership(household: household, role: :owner, email: 'health-event-owner@example.test')
    event, medication = create_health_event_medication(household: household)

    exporter = described_class.new(household: household, membership: owner, passphrase: passphrase)
    payload = exporter.payload
    mobile_payload = exporter.mobile_payload

    expect(payload.dig(:records, :health_events)).to be_nil
    expect(payload.dig(:records, :medications)).to be_empty
    expect(mobile_payload.dig(:records, :health_events).pluck(:portable_id)).to contain_exactly(event.portable_id)
    expect(mobile_payload.dig(:records, :medications).pluck(:portable_id)).to contain_exactly(medication.portable_id)
  end

  it 'limits member exports to people with manage grants' do
    household = create(:household)
    member = membership(household: household, role: :member, email: 'member@example.test')
    managed = create_portable_person_graph(household: household, name: 'Managed Patient',
                                           medication_name: 'Managed Medicine')
    create_portable_person_graph(household: household, name: 'Unmanaged Patient',
                                 medication_name: 'Unmanaged Medicine')
    grant_manage_access(member, managed)

    payload = export_payload(household: household, membership: member)

    expect(payload.dig('records', 'people').pluck('portable_id')).to contain_exactly(managed.portable_id)
    expect(payload.dig('records', 'medications').pluck('name')).to contain_exactly('Managed Medicine')
  end

  it 'limits owner exports to requested person IDs' do
    household = create(:household)
    owner = membership(household: household, role: :owner, email: 'filtered-owner@example.test')
    included = create_portable_person_graph(household: household, name: 'Included Patient',
                                            medication_name: 'Included Medicine')
    create_portable_person_graph(household: household, name: 'Excluded Patient',
                                 medication_name: 'Excluded Medicine')

    envelope = described_class.new(
      household: household,
      membership: owner,
      passphrase: passphrase,
      person_ids: [included.id]
    ).call
    payload = PortableData::Encryptor.decrypt(envelope, passphrase: passphrase)

    expect(payload.dig('records', 'people').pluck('portable_id')).to contain_exactly(included.portable_id)
    expect(payload.dig('records', 'medications').pluck('name')).to contain_exactly('Included Medicine')
  end

  it 'exports no people without a membership access context' do
    household = create(:household)
    create_portable_person_graph(household: household, name: 'No Context Patient',
                                 medication_name: 'No Context Medicine')

    payload = described_class.new(household: household, membership: nil, passphrase: passphrase).payload

    expect(payload.dig(:records, :people)).to be_empty
    expect(payload.dig(:records, :medications)).to be_empty
  end

  it 'records redacted export audit metadata' do
    household = create(:household)
    owner = membership(household: household, role: :owner, email: 'auditor@example.test')
    create_portable_person_graph(
      household: household,
      name: 'Audit Patient',
      medication_name: 'Sensitive Export Medicine'
    )

    expect do
      described_class.new(household: household, membership: owner, passphrase: passphrase).call
    end.to change(SecurityAuditEvent, :count).by(1)

    event = SecurityAuditEvent.order(:created_at).last
    expect(event.event_type).to eq('portable_data.exported')
    expect(event.metadata.to_json).not_to include('Sensitive Export Medicine')
    expect(event.metadata).to include('record_counts')
  end
end
