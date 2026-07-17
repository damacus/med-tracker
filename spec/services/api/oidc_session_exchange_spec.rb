# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::OidcSessionExchange do
  include ApiRequestHelpers

  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:jane) }
  let(:account) { user.person.account }
  let(:nonce) { "atomic-#{SecureRandom.hex(4)}" }
  let(:claims) do
    {
      'iss' => 'https://issuer.example.test',
      'sub' => 'jane-oidc-sub',
      'nonce' => nonce,
      'iat' => Time.current.to_i
    }
  end
  let(:provider_client) do
    instance_double(
      Api::OidcProviderClient,
      exchange_code: 'signed-id-token',
      decode_id_token: claims
    )
  end

  def request
    @request ||= instance_double(
      ActionDispatch::Request,
      user_agent: 'RSpec',
      remote_ip: '127.0.0.1',
      request_id: SecureRandom.uuid
    )
  end

  def params
    {
      authorization_code: 'authorization-code',
      code_verifier: 'a' * 64,
      redirect_uri: 'https://mobile.example.test/callback',
      nonce: nonce
    }
  end

  before do
    AccountLockout.where(account_id: account.id).delete_all
    AccountIdentity.find_or_create_by!(account: account, provider: 'oidc', uid: 'jane-oidc-sub')
    ensure_api_household_for(user)
  end

  it 'does not consume a nonce before identity and membership validation succeeds' do
    claims['sub'] = 'unlinked-subject'

    expect_nonce_not_consumed { call_exchange }

    claims['sub'] = 'jane-oidc-sub'
    account.household_memberships.each(&:suspended!)

    expect_nonce_not_consumed { call_exchange }
  end

  it 'rolls back nonce consumption when session issuance fails' do
    allow(ApiSession).to receive(:issue_for).and_raise(ActiveRecord::RecordInvalid.new(ApiSession.new))

    expect_nonce_not_consumed { call_exchange }
  end

  it 'rolls back nonce consumption when selection-grant issuance fails' do
    second_household = create(:household)
    second_household.household_memberships.create!(account: account, role: :member, status: :active)
    allow(ApiHouseholdSelectionGrant).to receive(:issue_for)
      .and_raise(ActiveRecord::RecordInvalid.new(ApiHouseholdSelectionGrant.new))

    expect_nonce_not_consumed { call_exchange }
  end

  def call_exchange
    described_class.new(params: params, request: request, provider_client: provider_client).call
  end

  def expect_nonce_not_consumed(&)
    nonce_count = ApiOidcNonce.count

    expect(&).to raise_error(described_class::Error)
    expect(ApiOidcNonce.count).to eq(nonce_count)
  end
end
