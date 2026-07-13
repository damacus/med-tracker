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
    end.to change {
      SecurityAuditEvent.where(event_type: 'api/admin/household_settings/updated').count
    }.by(1)

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'name')).to eq('API Admin Renamed Household')
    audit_event = SecurityAuditEvent.where(event_type: 'api/admin/household_settings/updated').order(:created_at).last
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

  it 'lists app tokens with nullable and revoked timestamps' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
    active_token, = ApiAppToken.issue_for(
      account: user.person.account,
      household_membership: api_session.household_membership,
      name: 'Active listed token'
    )
    revoked_token, = ApiAppToken.issue_for(
      account: user.person.account,
      household_membership: api_session.household_membership,
      name: 'Revoked listed token'
    )
    revoked_token.update!(last_used_at: 1.hour.ago, revoked_at: Time.current)

    get api_v1_household_admin_app_tokens_path(household_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    payloads = response.parsed_body.fetch('data').index_by { |token| token.fetch('id') }
    expect(payloads.fetch(active_token.id)).to include('revoked_at' => nil)
    expect(payloads.fetch(active_token.id).fetch('last_used_at')).to be_present
    expect(payloads.fetch(revoked_token.id).fetch('last_used_at')).to be_present
    expect(payloads.fetch(revoked_token.id).fetch('revoked_at')).to be_present
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
    expect(SecurityAuditEvent.order(:id).last.metadata.to_json).not_to include('token')

    delete api_v1_household_admin_invitation_path(household_id, invitation_id), headers: headers, as: :json

    expect(response).to have_http_status(:no_content)
    expect(HouseholdInvitation.find(invitation_id)).to be_revoked
  end

  it 'cannot read or revoke another household invitation' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
    other_account = Account.create!(email: 'api.other.invitation.owner@example.test', status: :verified)
    other_household = Household.create_with_owner!(
      name: 'API Other Invitation Household',
      owner_account: other_account,
      owner_person_attributes: {
        name: 'API Other Invitation Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
    foreign_invitation = other_household.household_invitations.create!(
      invited_by_membership: other_household.household_memberships.sole,
      email: 'api.foreign.invitation@example.test',
      membership_role: :member
    )

    get api_v1_household_admin_invitations_path(household_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').pluck('id')).not_to include(foreign_invitation.id)

    delete api_v1_household_admin_invitation_path(household_id, foreign_invitation.id), headers: headers, as: :json

    expect(response).to have_http_status(:not_found)
    expect(foreign_invitation.reload.revoked_at).to be_nil
  end

  it 'returns validation errors for invalid invitations and lists revoked timestamps' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)

    post api_v1_household_admin_invitations_path(household_id),
         params: { household_invitation: { email: '', membership_role: 'member' } },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)

    invitation = Household.find(household_id).household_invitations.create!(
      email: 'revoked.invitation@example.test',
      membership_role: 'member',
      invited_by_membership: api_session.household_membership,
      revoked_at: Time.current
    )

    get api_v1_household_admin_invitations_path(household_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    payload = response.parsed_body.fetch('data').find { |item| item.fetch('id') == invitation.id }
    expect(payload.fetch('revoked_at')).to be_present
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

  it 'refuses to revoke relationship-owned person access grants' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
    household = Household.find(household_id)
    carer_account = Account.create!(email: 'api.delegated.carer@example.test', status: :verified)
    carer = create(:person, household: household, account: carer_account)
    relationship = CareDelegation::Assign.new(
      carer: carer,
      patient: people(:john),
      relationship_type: :parent,
      granted_by_membership: api_session.household_membership
    ).call
    grant = relationship.person_access_grants.sole

    delete api_v1_household_admin_person_access_grant_path(household_id, grant.id), headers: headers, as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'errors', 'base'))
      .to include('Relationship-owned grants must be revoked through their carer relationship')
    expect(relationship.reload).to be_active
    expect(grant.reload.revoked_at).to be_nil
  end

  it 'invalidates target credentials after API membership changes' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
    target_login = api_login(users(:jane), household_id: household_id)
    target_session = ApiSession.lookup_by_access_token(target_login.fetch('access_token'))
    target_membership = target_session.household_membership
    app_token, app_token_value = ApiAppToken.issue_for(
      account: target_session.account,
      household_membership: target_membership,
      name: 'Before API membership change'
    )
    oauth_grant, oauth_token = issue_oauth_grant(target_membership)

    patch api_v1_household_admin_membership_path(household_id, target_membership.id),
          params: { household_membership: { role: 'administrator' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:ok)
    expect(target_membership.reload.permissions_version).to eq(target_session.permissions_version + 1)
    expect(app_token.reload).not_to be_active_for_membership
    expect(oauth_grant.reload).not_to be_active_for_membership
    [target_login.fetch('access_token'), app_token_value, oauth_token].each do |token|
      get api_v1_household_me_path(household_id), headers: api_auth_headers(token), as: :json
      expect(response).to have_http_status(:unauthorized)
    end
  end

  it 'invalidates target credentials after API grant creation and revocation' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
    target_login = api_login(users(:jane), household_id: household_id)
    target_session = ApiSession.lookup_by_access_token(target_login.fetch('access_token'))
    target_membership = target_session.household_membership
    target_app_token, target_app_token_value = ApiAppToken.issue_for(
      account: target_session.account,
      household_membership: target_membership,
      name: 'Before API grant creation'
    )
    target_oauth_grant, target_oauth_token = issue_oauth_grant(target_membership)

    post api_v1_household_admin_person_access_grants_path(household_id),
         params: {
           person_access_grant: {
             household_membership_id: target_membership.id,
             person_id: people(:john).id,
             access_level: 'manage',
             relationship_type: 'carer'
           }
         },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:created)
    grant_id = response.parsed_body.dig('data', 'id')
    expect(target_membership.reload.permissions_version).to eq(target_session.permissions_version + 1)
    expect(target_session.reload).not_to be_active_for_membership
    expect(target_app_token.reload).not_to be_active_for_membership
    expect(target_oauth_grant.reload).not_to be_active_for_membership
    expect_rejected_api_tokens(target_login.fetch('access_token'), target_app_token_value, target_oauth_token)

    fresh_session, fresh_session_token = ApiSession.issue_for(
      account: target_session.account,
      household_membership: target_membership
    )
    fresh_app_token, fresh_app_token_value = ApiAppToken.issue_for(
      account: target_session.account,
      household_membership: target_membership,
      name: 'Before API grant revocation'
    )
    fresh_oauth_grant, fresh_oauth_token = issue_oauth_grant(target_membership)
    delete api_v1_household_admin_person_access_grant_path(household_id, grant_id), headers: headers, as: :json

    expect(response).to have_http_status(:no_content)
    expect(target_membership.reload.permissions_version).to eq(fresh_session.permissions_version + 1)
    expect(fresh_session.reload).not_to be_active_for_membership
    expect(fresh_app_token.reload).not_to be_active_for_membership
    expect(fresh_oauth_grant.reload).not_to be_active_for_membership
    expect_rejected_api_tokens(fresh_session_token, fresh_app_token_value, fresh_oauth_token)
  end

  it 'rejects person grants targeting a membership in another household' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
    foreign_household = Household.create!(name: 'Foreign API Access', slug: "foreign-api-#{SecureRandom.hex(4)}")
    foreign_account = Account.create!(email: "foreign-api-#{SecureRandom.hex(4)}@example.test", status: :verified)
    foreign_person = foreign_household.people.create!(
      account: foreign_account,
      name: 'Foreign API Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
    foreign_membership = foreign_household.household_memberships.create!(
      account: foreign_account,
      person: foreign_person,
      role: :owner,
      status: :active
    )

    expect do
      post api_v1_household_admin_person_access_grants_path(household_id),
           params: {
             person_access_grant: {
               household_membership_id: foreign_membership.id,
               person_id: people(:john).id,
               access_level: 'manage',
               relationship_type: 'carer'
             }
           },
           headers: headers,
           as: :json
    end.not_to(change { foreign_membership.reload.permissions_version })

    expect(response).to have_http_status(:unprocessable_content)
    event = SecurityAuditEvent.where(event_type: 'household_access.person_grant_changed').order(:id).last
    expect(event.metadata).to include('outcome' => 'rejected', 'target_membership_id' => foreign_membership.id)
  end

  it 'returns validation errors for invalid person access grants and lists revoked timestamps' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)

    post api_v1_household_admin_person_access_grants_path(household_id),
         params: { person_access_grant: { access_level: 'manage' } },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)

    member_account = Account.create!(email: 'api.revoked.grant@example.test', status: :verified)
    membership = Household.find(household_id).household_memberships.create!(account: member_account, role: :member)
    grant = PersonAccessGrant.create!(
      household_id: household_id,
      household_membership: membership,
      person: people(:john),
      access_level: 'manage',
      relationship_type: 'carer',
      granted_by_membership: api_session.household_membership,
      expires_at: 1.week.from_now,
      revoked_at: Time.current
    )

    get api_v1_household_admin_person_access_grants_path(household_id), headers: headers, as: :json

    expect(response).to have_http_status(:ok)
    payload = response.parsed_body.fetch('data').find { |item| item.fetch('id') == grant.id }
    expect(payload.fetch('expires_at')).to be_present
    expect(payload.fetch('revoked_at')).to be_present
  end

  it 'returns validation errors for invalid household settings and membership updates' do
    api_session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)

    patch api_v1_household_admin_settings_path(household_id),
          params: { household: { name: '' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:unprocessable_content)

    membership = Household.find(household_id)
                          .household_memberships
                          .where.not(id: api_session.household_membership_id)
                          .first

    patch api_v1_household_admin_membership_path(household_id, membership.id),
          params: { household_membership: { role: '' } },
          headers: headers,
          as: :json

    expect(response).to have_http_status(:unprocessable_content)
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

  def issue_oauth_grant(membership)
    raw_token = "oauth-#{SecureRandom.hex(24)}"
    grant = OauthGrant.create!(
      account: membership.account,
      oauth_application: oauth_application(membership),
      household_membership: membership,
      person: membership.person,
      permissions_version: membership.permissions_version,
      expires_in: 1.hour.from_now,
      scopes: 'patient/*.rs',
      token_hash: OauthGrant.digest(raw_token)
    )
    [grant, raw_token]
  end

  def expect_rejected_api_tokens(*tokens)
    tokens.each do |token|
      get api_v1_household_me_path(household_id), headers: api_auth_headers(token), as: :json
      expect(response).to have_http_status(:unauthorized)
    end
  end

  def oauth_application(membership)
    OauthApplication.create!(
      account: membership.account,
      name: 'API access change client',
      client_id: "api-access-change-#{SecureRandom.hex(8)}",
      redirect_uri: 'https://client.example/callback',
      scopes: 'patient/*.rs'
    )
  end
end
