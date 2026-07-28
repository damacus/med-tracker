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
    expect(result.response_headers).to eq({})
  end

  it 'rejects replay headers from a different authenticated account' do
    request = request_double(method: 'POST', key: 'account-scoped-key')
    response = response_double(
      status: 201,
      body: '{"data":{"name":"Private result"}}',
      headers: { 'ETag' => '"private-etag"' }
    )
    described_class.new(request: request, credential: api_session, household: household).store!(response)
    other_session = api_session_for(create_account_with_membership(household))

    result = described_class.new(request: request, credential: other_session, household: household).lookup

    expect(result.replayed).to be false
    expect(result.conflict).to be true
    expect(result.response_headers).to eq({})
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

    expect do
      described_class.new(request: request, credential: api_session, household: household).store!(private_response)
    end.to change(ApiIdempotencyKey, :count).by(1)

    key = ApiIdempotencyKey.order(:id).last
    expect(key).to have_attributes(stored_response_attributes)
  end

  it 'applies the current allowlist when reading stored response headers' do
    store = store_for('read-allowlist-key')
    store.store!(
      response_double(status: 201, body: '{"ok":true}', headers: { 'ETag' => '"stored-etag"' })
    )
    ApiIdempotencyKey.find_by!(household: household, key: 'read-allowlist-key').update!(
      response_headers: { 'ETag' => '"stored-etag"', 'Set-Cookie' => 'private=value' }
    )

    result = store.lookup

    expect(result.replayed).to be true
    expect(result.response_headers).to eq('ETag' => '"stored-etag"')
  end

  it 'replays an unexpired legacy response without inventing headers' do
    store = store_for('legacy-headerless-key')
    store.store!(response_double(status: 201, body: '{"legacy":true}'))
    executed = false

    result = store.with_reservation(
      response: response_double(status: 201, body: '{"replacement":true}', headers: { 'ETag' => '"new-etag"' })
    ) { executed = true }

    expect(executed).to be false
    expect(result.replayed).to be true
    expect(result.response_headers).to eq({})
  end

  it 'replaces an expired response while holding the reservation' do
    store = store_for('expired-key')
    store.store!(response_double(status: 201, body: '{"expired":true}'))
    expired_record = ApiIdempotencyKey.find_by!(household: household, key: 'expired-key')
    expired_record.update!(expires_at: 1.minute.ago)
    name = replace_expired_response(store)
    replacement = ApiIdempotencyKey.find_by!(household: household, key: 'expired-key')

    expect(replacement.id).not_to eq(expired_record.id)
    expect(replacement).to have_attributes(
      response_body: { 'replacement' => true },
      response_headers: { 'ETag' => '"replacement-etag"' }
    )
    expect(Person.exists?(household: household, name: name)).to be true
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
    other_household = Household.create!(
      name: 'Other Idempotency Store Spec',
      slug: "other-idempotency-store-#{SecureRandom.hex(4)}"
    )
    other_session = api_session_for(create_account_with_membership(other_household))
    first_store = described_class.new(request: request, credential: api_session, household: household)
    second_store = described_class.new(request: request, credential: other_session, household: other_household)

    store_household_response(first_store, number: 1)
    store_household_response(second_store, number: 2)

    expect(ApiIdempotencyKey.where(key: 'household-scoped-key').count).to eq(2)
    expect(first_store.lookup.response_headers).to eq('ETag' => '"household-one"')
    expect(second_store.lookup.response_headers).to eq('ETag' => '"household-two"')
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

  def response_double(status:, body:, headers: {})
    instance_double(ActionDispatch::Response, status: status, body: body, headers: headers)
  end

  def private_response
    response_double(
      status: 201,
      body: 'not-json',
      headers: {
        'ETag' => '"record-etag"',
        'Set-Cookie' => 'private=value',
        'X-Request-Id' => 'delivery-specific',
        'Retry-After' => '30'
      }
    )
  end

  def stored_response_attributes
    {
      household: household,
      account: account,
      api_session: api_session,
      response_status: 201,
      response_body: {},
      response_headers: { 'ETag' => '"record-etag"' }
    }
  end

  def replace_expired_response(store)
    name = "Replacement Person #{SecureRandom.hex(4)}"
    response = response_double(
      status: 201,
      body: '{"replacement":true}',
      headers: { 'ETag' => '"replacement-etag"' }
    )
    store.with_reservation(response: response) { create_unowned_person(name) }
    name
  end

  def store_household_response(store, number:)
    store.store!(
      response_double(
        status: 201,
        body: %({"household":#{number}}),
        headers: { 'ETag' => %("household-#{number == 1 ? 'one' : 'two'}") }
      )
    )
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
