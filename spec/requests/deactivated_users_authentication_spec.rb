# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Deactivated user authentication' do
  fixtures :accounts, :people, :users

  let(:user) { users(:jane) }
  let(:account) { accounts(:jane_doe) }

  it 'does not establish a Rodauth session for an inactive user' do
    user.deactivate!

    post '/login', params: { email: account.email, password: 'password' }

    expect(session[:account_id]).to be_blank
    expect(response).not_to redirect_to(dashboard_path)
  end

  it 'revokes access for an inactive user with an existing session' do
    sign_in(user)
    expect(session[:account_id]).to be_present

    user.deactivate!
    get dashboard_path

    expect(response).to redirect_to(login_path)
    expect(session[:account_id]).to be_blank
  end

  it 'revokes API credentials when a user is deactivated' do
    household = ensure_api_household_for(user)
    membership = household.household_memberships.find_by!(account: account)
    api_session, = ApiSession.issue_for(account: account, household_membership: membership)
    api_app_token, raw_token = ApiAppToken.issue_for(
      account: account,
      household_membership: membership,
      name: 'Deactivation regression token'
    )

    user.deactivate!

    expect(api_session.reload.revoked_at).to be_present
    expect(api_app_token.reload.revoked_at).to be_present

    get api_v1_auth_households_path, headers: api_auth_headers(raw_token), as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'deactivates and revokes API credentials when token audit persistence fails' do
    household = ensure_api_household_for(user)
    membership = household.household_memberships.find_by!(account: account)
    api_session, = ApiSession.issue_for(account: account, household_membership: membership)
    api_app_token, = ApiAppToken.issue_for(
      account: account,
      household_membership: membership,
      name: 'Audit failure regression token'
    )
    allow(Audit::VersionEvent).to receive(:record!) do
      ActiveRecord::Base.connection.execute('SELECT missing_audit_column FROM users')
    end

    expect { user.deactivate! }.not_to raise_error

    expect(user.reload).not_to be_active
    expect(api_session.reload.revoked_at).to be_present
    expect(api_app_token.reload.revoked_at).to be_present
  end
end
