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
