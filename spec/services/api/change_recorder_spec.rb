# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::ChangeRecorder do
  fixtures :accounts

  let(:account) { accounts(:jane_doe) }
  let(:household) { Household.create!(name: 'Change Recorder Spec', slug: "change-recorder-#{SecureRandom.hex(4)}") }
  let(:person) { create_person_for(account, household) }
  let(:membership) do
    household.household_memberships.create!(account: account, person: person, role: :owner, status: :active)
  end
  let(:credential) { instance_double(ApiSession, account: account) }

  it 'records persisted record changes with portable metadata' do
    change_recorder.record(person, action: 'updated')

    expect(ApiChangeEvent.count).to eq(1)

    event = ApiChangeEvent.order(:id).last
    expect(event).to have_attributes(
      household: household,
      account: account,
      household_membership: membership,
      request_id: 'request-123',
      record_type: 'Person',
      record_id: person.id,
      record_portable_id: person.portable_id,
      action: 'updated'
    )
    expect(event.metadata).to include('portable_id' => person.portable_id)
  end

  it 'skips changes without the required tenant and record context' do
    expect do
      invalid_contexts.each { |context| record_context(context) }
    end.not_to change(ApiChangeEvent, :count)
  end

  it 'records persisted records that do not expose portable IDs' do
    change_recorder.record(account, action: 'updated')

    event = ApiChangeEvent.order(:id).last
    expect(event.record_type).to eq('Account')
    expect(event.record_portable_id).to be_nil
    expect(event.metadata).not_to have_key('portable_id')
  end

  def create_person_for(account, household)
    Person.create!(
      household: household,
      account: account,
      name: 'Change Recorder Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def change_recorder
    described_class.new(household: household, credential: credential, membership: membership, request: request)
  end

  def request
    instance_double(ActionDispatch::Request, request_id: 'request-123')
  end

  def invalid_contexts
    [
      missing_household_context,
      missing_credential_context,
      blank_credential_context,
      missing_membership_context,
      missing_record_context,
      unpersisted_record_context
    ]
  end

  def missing_household_context
    { household: nil, credential: credential, membership: membership, record: person }
  end

  def missing_credential_context
    { household: household, credential: nil, membership: membership, record: person }
  end

  def blank_credential_context
    blank_credential = instance_double(ApiSession, account: nil)

    { household: household, credential: blank_credential, membership: membership, record: person }
  end

  def missing_membership_context
    { household: household, credential: credential, membership: nil, record: person }
  end

  def missing_record_context
    { household: household, credential: credential, membership: membership, record: nil }
  end

  def unpersisted_record_context
    { household: household, credential: credential, membership: membership, record: Person.new }
  end

  def record_context(context)
    described_class.new(
      household: context.fetch(:household),
      credential: context.fetch(:credential),
      membership: context.fetch(:membership),
      request: request
    ).record(context.fetch(:record), action: 'updated')
  end
end
