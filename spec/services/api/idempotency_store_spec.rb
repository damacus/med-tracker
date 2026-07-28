# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::IdempotencyStore do
  fixtures :accounts

  let(:account) { accounts(:jane_doe) }
  let(:household) do
    Household.create!(name: 'Idempotency Store Spec', slug: "idempotency-store-#{SecureRandom.hex(4)}")
  end
  let(:person) { create_person_for(account, household) }
  let(:membership) do
    household.household_memberships.create!(account: account, person: person, role: :owner, status: :active)
  end
  let(:api_session) { ApiSession.issue_for(account: account, household_membership: membership).first }

  it 'is active only for keyed mutating requests with tenant and credential context' do
    active_request = request_double(method: 'POST', key: 'active-key')

    expect(described_class.new(request: active_request, credential: api_session, household: household)).to be_active
    expect(described_class.new(request: request_double(method: 'GET', key: 'active-key'),
                               credential: api_session, household: household)).not_to be_active
    expect(described_class.new(request: request_double(method: 'POST', key: nil),
                               credential: api_session, household: household)).not_to be_active
    expect(described_class.new(request: active_request, credential: nil, household: household)).not_to be_active
    expect(described_class.new(request: active_request, credential: api_session, household: nil)).not_to be_active
  end

  it 'returns an empty lookup result when no key has been stored' do
    result = described_class.new(
      request: request_double(method: 'POST', key: 'missing-key'),
      credential: api_session,
      household: household
    ).lookup

    expect(result.record).to be_nil
    expect(result.replayed).to be false
    expect(result.conflict).to be false
  end

  it 'rejects replay from a different authenticated account' do
    request = request_double(method: 'POST', key: 'account-scoped-key')
    response = response_double(status: 201, body: '{"data":{"name":"Private result"}}')
    described_class.new(request: request, credential: api_session, household: household).store!(response)
    other_session = api_session_for(create_account_with_membership(household))

    result = described_class.new(request: request, credential: other_session, household: household).lookup

    expect(result.replayed).to be false
    expect(result.conflict).to be true
  end

  it 'derives a stable opaque reservation id from household and key' do
    first = store_for('private-key-material')
    matching = store_for('private-key-material')
    different = store_for('different-key-material')

    expect(first.send(:reservation_id)).to be_a(Integer)
    expect(first.send(:reservation_id)).to eq(matching.send(:reservation_id))
    expect(first.send(:reservation_id)).not_to eq(different.send(:reservation_id))
    expect(first.send(:reservation_id).to_s).not_to include('private-key-material')
  end

  it 'stores non-PHI response metadata and normalises invalid JSON response bodies' do
    request = request_double(method: 'POST', key: 'store-key')
    response = response_double(status: 201, body: 'not-json')

    expect do
      described_class.new(request: request, credential: api_session, household: household).store!(response)
    end.to change(ApiIdempotencyKey, :count).by(1)

    key = ApiIdempotencyKey.order(:id).last
    expect(key).to have_attributes(
      household: household,
      account: account,
      api_session: api_session,
      response_status: 201,
      response_body: {}
    )
  end

  it 'stores app-token idempotency attribution' do
    app_token = ApiAppToken.issue_for(
      account: account,
      household_membership: membership,
      name: 'Idempotency app token'
    ).first
    request = request_double(method: 'PATCH', key: 'app-token-key')
    response = response_double(status: 200, body: '{"ok":true}')

    described_class.new(request: request, credential: app_token, household: household).store!(response)

    key = ApiIdempotencyKey.order(:id).last
    expect(key.api_session).to be_nil
    expect(key.api_app_token).to eq(app_token)
  end

  it 'does not store inactive or failed responses' do
    inactive_request = request_double(method: 'GET', key: 'inactive-key')
    failed_response = response_double(status: 500, body: '{}')

    expect do
      described_class.new(
        request: inactive_request,
        credential: api_session,
        household: household
      ).store!(failed_response)
      described_class.new(request: request_double(method: 'POST', key: 'failed-key'),
                          credential: api_session, household: household).store!(failed_response)
    end.not_to change(ApiIdempotencyKey, :count)
  end

  it 'raises when a response cannot be stored uniquely' do
    request = request_double(method: 'POST', key: 'duplicate-key')
    response = response_double(status: 201, body: '{"ok":true}')
    store = described_class.new(request: request, credential: api_session, household: household)
    store.store!(response)

    expect { store.store!(response) }.to raise_error(ActiveRecord::RecordInvalid)
  end

  it 'rolls back a rendered server error and leaves the key retryable' do
    request = request_double(method: 'POST', key: 'server-error-key')
    store = described_class.new(request: request, credential: api_session, household: household)
    name = "Rolled Back Person #{SecureRandom.hex(4)}"

    mutate_person(store, status: 500, name: name)
    expect(Person.exists?(household: household, name: name)).to be false
    expect(ApiIdempotencyKey.exists?(household: household, key: 'server-error-key')).to be false

    mutate_person(store, status: 201, name: name)
    expect(Person.exists?(household: household, name: name)).to be true
    expect(ApiIdempotencyKey.exists?(household: household, key: 'server-error-key')).to be true
  end

  it 'rolls back a raised exception and leaves the key retryable' do
    request = request_double(method: 'POST', key: 'exception-key')
    store = described_class.new(request: request, credential: api_session, household: household)
    name = "Exception Person #{SecureRandom.hex(4)}"

    expect { mutate_person_then_raise(store, name) }.to raise_error('mutation failed')
    expect(Person.exists?(household: household, name: name)).to be false
    expect(ApiIdempotencyKey.exists?(household: household, key: 'exception-key')).to be false
  end

  it 'allows the same key to be stored independently in different households' do
    request = request_double(method: 'POST', key: 'household-scoped-key')
    response = response_double(status: 201, body: '{"ok":true}')
    other_household = Household.create!(
      name: 'Other Idempotency Store Spec',
      slug: "other-idempotency-store-#{SecureRandom.hex(4)}"
    )
    other_session = api_session_for(create_account_with_membership(other_household))

    described_class.new(request: request, credential: api_session, household: household).store!(response)
    described_class.new(request: request, credential: other_session, household: other_household).store!(response)

    expect(ApiIdempotencyKey.where(key: 'household-scoped-key').count).to eq(2)
  end

  def request_double(method:, key:)
    instance_double(
      ActionDispatch::Request,
      headers: { 'Idempotency-Key' => key },
      request_method: method,
      path: '/api/v1/households/1/sync/batches',
      filtered_parameters: { 'controller' => 'sync', 'action' => 'create', 'payload' => { 'safe' => true } },
      post?: method == 'POST',
      patch?: method == 'PATCH',
      put?: method == 'PUT',
      delete?: method == 'DELETE'
    )
  end

  def response_double(status:, body:)
    instance_double(ActionDispatch::Response, status: status, body: body)
  end

  def store_for(key)
    described_class.new(
      request: request_double(method: 'POST', key: key),
      credential: api_session,
      household: household
    )
  end

  def mutate_person(store, status:, name:)
    store.with_reservation(response: response_double(status: status, body: '{"ok":true}')) do
      create_unowned_person(name)
    end
  end

  def mutate_person_then_raise(store, name)
    store.with_reservation(response: response_double(status: 201, body: '{"ok":true}')) do
      create_unowned_person(name)
      raise 'mutation failed'
    end
  end

  def create_unowned_person(name)
    Person.create!(
      household: household,
      name: name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def create_account_with_membership(target_household)
    other_account = Account.create!(
      email: "idempotency-other-#{SecureRandom.hex(4)}@example.test",
      status: :verified
    )
    other_person = create_person_for(other_account, target_household)
    target_household.household_memberships.create!(
      account: other_account,
      person: other_person,
      role: :owner,
      status: :active
    )
  end

  def api_session_for(target_membership)
    ApiSession.issue_for(account: target_membership.account, household_membership: target_membership).first
  end

  def create_person_for(account, household)
    Person.create!(
      household: household,
      account: account,
      name: 'Idempotency Store Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end
end
