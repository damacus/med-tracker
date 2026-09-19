# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AUTH: OmniAuth Auto Linking', type: :system do
  fixtures :accounts, :people, :users

  before do
    skip 'OIDC not configured' unless ENV.fetch('OIDC_CLIENT_ID', nil).present? &&
                                      ENV.fetch('OIDC_ISSUER_URL', nil).present?

    OmniAuth.config.test_mode = true
  end

  after do
    OmniAuth.config.test_mode = false
    OmniAuth.config.mock_auth[:oidc] = nil
  end

  it 'auto-links an existing account based on email when logging in via OmniAuth' do
    existing_account = accounts(:damacus)
    email = existing_account.email

    # Verify no identities exist yet
    expect(identity_count(account_id: existing_account.id)).to eq(0)

    # Mock the OmniAuth response
    OmniAuth.config.mock_auth[:oidc] = OmniAuth::AuthHash.new(
      provider: 'oidc',
      uid: '12345',
      info: {
        email: email,
        name: 'Damacus Test'
      }
    )

    # In a real setup, there would be a button to login with OIDC
    # We can simulate the callback directly or click the button if it exists
    visit '/auth/oidc/callback'

    # Should be logged in and redirected to dashboard
    expect(page).to have_current_path(%r{\A/households/[^/]+/dashboard\z})

    # Should have created an identity linked to the existing account
    expect(identity_count(account_id: existing_account.id, provider: 'oidc', uid: '12345')).to eq(1)
  end

  def identity_count(account_id:, provider: nil, uid: nil)
    conditions = { account_id:, provider:, uid: }.compact
    quoted_conditions = conditions.map do |column, value|
      "#{column} = #{ActiveRecord::Base.connection.quote(value)}"
    end
    sql = "SELECT COUNT(*) FROM account_identities WHERE #{quoted_conditions.join(' AND ')}"

    ActiveRecord::Base.connection.select_value(sql)
  end
end
