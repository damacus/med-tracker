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

  it 'stores non-PHI response metadata and normalises invalid JSON response bodies' do
    request = request_double(method: 'POST', key: 'store-key')
    response = instance_double(ActionDispatch::Response, status: 201, body: 'not-json')

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
    response = instance_double(ActionDispatch::Response, status: 200, body: '{"ok":true}')

    described_class.new(request: request, credential: app_token, household: household).store!(response)

    key = ApiIdempotencyKey.order(:id).last
    expect(key.api_session).to be_nil
    expect(key.api_app_token).to eq(app_token)
  end

  it 'does not store inactive or failed responses' do
    inactive_request = request_double(method: 'GET', key: 'inactive-key')
    failed_response = instance_double(ActionDispatch::Response, status: 500, body: '{}')

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

  it 'ignores duplicate stores for the same key' do
    request = request_double(method: 'POST', key: 'duplicate-key')
    response = instance_double(ActionDispatch::Response, status: 201, body: '{"ok":true}')
    store = described_class.new(request: request, credential: api_session, household: household)

    expect do
      2.times { store.store!(response) }
    end.to change(ApiIdempotencyKey, :count).by(1)
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
