require 'rails_helper'

RSpec.describe 'AUTH: OmniAuth Auto Linking', type: :system do
  fixtures :accounts, :people, :users

  before do
    # Enable OmniAuth test mode
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
    expect(AccountIdentity.where(account_id: existing_account.id).count).to eq(0)

    # Mock the OmniAuth response
    OmniAuth.config.mock_auth[:oidc] = OmniAuth::AuthHash.new({
      provider: 'oidc',
      uid: '12345',
      info: {
        email: email,
        name: 'Damacus Test'
      }
    })

    # In a real setup, there would be a button to login with OIDC
    # We can simulate the callback directly or click the button if it exists
    visit '/auth/oidc/callback'

    # Should be logged in and redirected to dashboard
    expect(page).to have_current_path('/dashboard')
    
    # Should have created an identity linked to the existing account
    expect(AccountIdentity.where(account_id: existing_account.id, provider: 'oidc', uid: '12345').count).to eq(1)
  end
end
