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

  it 'skips changes without household or portable record context' do
    expect do
      missing_household_contexts.each { |context| record_context(context) }
    end.not_to change(ApiChangeEvent, :count)
  end

  it 'records system changes without an actor account or membership' do
    described_class.new(
      household: household,
      account: nil,
      membership: nil,
      request_id: 'system-job'
    ).record(person, action: 'updated')

    event = ApiChangeEvent.order(:id).last
    expect(event).to have_attributes(
      record_type: 'Person',
      record_portable_id: person.portable_id,
      account: nil,
      household_membership: nil,
      request_id: 'system-job'
    )
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
    described_class.new(
      household: household,
      account: account,
      membership: membership,
      request_id: request.request_id
    )
  end

  def request
    instance_double(ActionDispatch::Request, request_id: 'request-123')
  end

  def missing_household_contexts
    [
      missing_household_context,
      missing_record_context,
      missing_portable_record_context
    ]
  end

  def missing_household_context
    { household: nil, account: account, membership: membership, record: person }
  end

  def missing_record_context
    { household: household, account: account, membership: membership, record: nil }
  end

  def missing_portable_record_context
    { household: household, account: account, membership: membership, record: account }
  end

  def record_context(context)
    described_class.new(
      household: context.fetch(:household),
      account: context.fetch(:account),
      membership: context.fetch(:membership),
      request_id: request.request_id
    ).record(context.fetch(:record), action: 'updated')
  end
end
