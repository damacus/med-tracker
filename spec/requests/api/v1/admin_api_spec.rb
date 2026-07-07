# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 household administration' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }
  let(:api_session) { ApiSession.lookup_by_access_token(login_data.fetch('access_token')) }

  it 'reads household settings and memberships for household managers' do
    get api_v1_household_admin_settings_path(household_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'id')).to eq(household_id)

    get api_v1_household_admin_memberships_path(household_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).not_to be_empty
  end

  it 'requires fresh privileged proof for admin mutations' do
    patch api_v1_household_admin_settings_path(household_id),
          params: { household: { name: 'API Admin Renamed Household' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:forbidden)
    expect(response.parsed_body.dig('error', 'code')).to eq('fresh_privileged_action_required')

    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)

    expect do
      patch api_v1_household_admin_settings_path(household_id),
            params: { household: { name: 'API Admin Renamed Household' } },
            headers: headers,
            as: :json
    end.to change(SecurityAuditEvent, :count).by(1)

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'name')).to eq('API Admin Renamed Household')
    audit_event = SecurityAuditEvent.order(:created_at).last
    expect(audit_event.metadata).not_to include('body', 'token', 'bundle')
  end

  it 'issues and revokes app tokens without storing the raw token in audit metadata' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)

    expect do
      post api_v1_household_admin_app_tokens_path(household_id),
           params: { api_app_token: { name: 'CLI app token' } },
           headers: headers,
           as: :json
    end.to change(ApiAppToken, :count).by(1)

    expect(response).to have_http_status(:created)
    raw_token = response.parsed_body.dig('data', 'token')
    app_token_id = response.parsed_body.dig('data', 'id')
    expect(raw_token).to start_with(ApiAppToken::TOKEN_PREFIX)

    security_metadata = SecurityAuditEvent.order(:created_at).last.metadata
    expect(security_metadata.to_json).not_to include(raw_token)

    delete api_v1_household_admin_app_token_path(household_id, app_token_id), headers: headers, as: :json

    expect(response).to have_http_status(:no_content)
    expect(ApiAppToken.find(app_token_id)).to be_revoked_at
  end

  it 'creates and revokes invitations without exposing invitation tokens' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)

    post api_v1_household_admin_invitations_path(household_id),
         params: { household_invitation: { email: 'api.invited@example.test', membership_role: 'member' } },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    invitation_id = response.parsed_body.dig('data', 'id')
    expect(response.parsed_body['data'].to_json).not_to include('token')

    delete api_v1_household_admin_invitation_path(household_id, invitation_id), headers: headers, as: :json

    expect(response).to have_http_status(:no_content)
    expect(HouseholdInvitation.find(invitation_id)).to be_revoked
  end

  it 'creates and revokes person access grants for managed members' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
    member_account = Account.create!(email: 'api.grant.member@example.test', status: :verified)
    membership = Household.find(household_id).household_memberships.create!(account: member_account, role: :member)

    post api_v1_household_admin_person_access_grants_path(household_id),
         params: {
           person_access_grant: {
             household_membership_id: membership.id,
             person_id: people(:john).id,
             access_level: 'manage',
             relationship_type: 'carer'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    grant_id = response.parsed_body.dig('data', 'id')
    expect(response.parsed_body.dig('data', 'access_level')).to eq('manage')

    delete api_v1_household_admin_person_access_grant_path(household_id, grant_id), headers: headers, as: :json

    expect(response).to have_http_status(:no_content)
    expect(PersonAccessGrant.find(grant_id)).to be_revoked_at
  end

  it 'lists household security audit events for managers only' do
    SecurityAuditEvent.create!(
      household_id: household_id,
      actor_account: user.person.account,
      actor_membership: api_session.household_membership,
      event_type: 'api/admin/test',
      metadata: { target_type: 'Household', outcome: 'success' },
      request_id: 'audit-list-test'
    )

    get api_v1_household_admin_audit_logs_path(household_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').pluck('event_type')).to include('api/admin/test')
  end

  it 'rejects household administration for non-manager members' do
    member_user = users(:jane)
    member_login = api_login(member_user, household_id: household_id)

    get api_v1_household_admin_settings_path(household_id),
        headers: api_auth_headers(member_login.fetch('access_token')),
        as: :json

    expect(response).to have_http_status(:forbidden)
  end
end
