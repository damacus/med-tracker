require 'rails_helper'

RSpec.describe 'API v1 invitation acceptance' do
  fixtures :all

  let(:login) { api_login(users(:jane)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:target) { create(:household) }
  let(:inviter) do
    target.household_memberships.create!(account: users(:admin).person.account, role: :administrator, status: :active)
  end
  let(:invitation) do
    target.household_invitations.create!(email: account.email, invited_by_membership: inviter, membership_role: :member)
  end

  before do
    login
    invitation
  end

  def account
    users(:jane).person.account
  end

  it 'advertises session-only online invitation acceptance' do
    get '/api/v1/capabilities', as: :json
    expect(response.parsed_body.dig('data', 'invitations')).to include(
      'actions' => ['accept'], 'online_only' => true, 'acceptance_session_required' => true
    )
  end

  def accept(token = invitation.plain_token)
    post '/api/v1/invitations/accept', params: { token: token }, headers: headers, as: :json
  end

  def accepted_membership
    target.household_memberships.find_by!(account: account)
  end

  it 'creates one membership and the intended grants for the verified identity' do
    patient = target.people.create!(name: 'Invited Patient', date_of_birth: 40.years.ago.to_date)
    invitation.household_invitation_grants.create!(household: target, person: patient, access_level: :record,
                                                   relationship_type: :professional)
    accept
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).to include(
      'household_id' => target.id.to_s, 'membership_id' => accepted_membership.id.to_s, 'role' => 'member'
    )
    expect(invitation.reload.accepted_at).to be_present
    expect(accepted_membership.person.account).to eq(account)
    expect(accepted_membership.person_access_grants.active.where(person: patient).sole.access_level).to eq('record')
    expect(CarerRelationship.find_by(carer: accepted_membership.person, patient: patient)).to be_present
    get api_v1_auth_households_path, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data').map { |household| household.fetch('id') }).to include(target.id)
  end

  it 'replays acceptance without duplicating or restoring changed grants' do
    token = invitation.plain_token
    accept(token)
    expect(response).to have_http_status(:ok)
    membership = accepted_membership
    membership.person_access_grants.find_each { |grant| grant.update!(revoked_at: Time.current) }
    expect { accept(token) }.not_to change(HouseholdMembership, :count)
    expect(response).to have_http_status(:ok)
    expect(membership.person_access_grants.active).to be_empty
  end

  it 'keeps the new household profile accessible with its own session' do
    accept
    expect(response).to have_http_status(:ok)
    _, target_token = ApiSession.issue_for(account: account, household_membership: accepted_membership)
    get "/api/v1/households/#{target.id}/profile", headers: api_auth_headers(target_token), as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'person_id')).to eq(accepted_membership.person_id.to_s)
    get api_v1_household_me_path(login.dig('household', 'id')), headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'id')).to eq(users(:jane).id)
  end

  it 'rejects expired, revoked, mismatched and unknown invitations without membership changes' do
    token = invitation.plain_token
    invitation.update!(expires_at: 1.second.ago)
    expect { accept(token) }.not_to change(HouseholdMembership, :count)
    expect(response).to have_http_status(:unprocessable_content)
    invitation.update!(expires_at: 1.day.from_now, revoked_at: Time.current)
    accept(token)
    expect(response).to have_http_status(:unprocessable_content)
    invitation.update!(revoked_at: nil, email: 'different@example.test')
    accept(token)
    expect(response).to have_http_status(:unprocessable_content)
    accept('unknown-token')
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'code')).to eq('invitation_unavailable')
    expect(response.body).not_to include(target.name, invitation.email)
  end

  it 'rejects a revoked accepted membership even when the request has an idempotency key' do
    token = invitation.plain_token
    headers['Idempotency-Key'] = SecureRandom.uuid
    accept(token)
    expect(response).to have_http_status(:ok)
    expect(account.reload.person.id).to eq(users(:jane).person.id)
    accepted_membership.update!(status: :revoked)
    accept(token)
    expect(response).to have_http_status(:unprocessable_content)
  end

  it 'rejects acceptance if the inviter no longer has household authority' do
    inviter.update!(status: :revoked)
    expect { accept }.not_to change(HouseholdMembership, :count)
    expect(response).to have_http_status(:unprocessable_content)
    expect(invitation.reload.accepted_at).to be_nil
  end

  it 'requires a verified authenticated account' do
    account.update!(status: :unverified)
    accept
    expect(response).to have_http_status(:unauthorized)
    expect(invitation.reload.accepted_at).to be_nil
  end

  it 'does not replace an existing household membership through an invitation' do
    membership = target.household_memberships.create!(account: account, role: :member, status: :suspended)
    accept
    expect(response).to have_http_status(:unprocessable_content)
    expect(membership.reload).to be_suspended
  end

  it 'does not let a household app token acquire another household membership' do
    membership = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    _, token = ApiAppToken.issue_for(account: account, household_membership: membership, name: 'Scoped integration')
    headers['Authorization'] = "Bearer #{token}"
    accept
    expect(response).to have_http_status(:forbidden)
    expect(invitation.reload.accepted_at).to be_nil
  end

  it 'accepts and replays using the forced-RLS application role' do
    token = invitation.plain_token
    account
    target
    ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
    accept(token)
    expect(response).to have_http_status(:ok)
    accept(token)
    expect(response).to have_http_status(:ok)
    expect(response.headers['Cache-Control']).to include('no-store')
  end

  it 'handles an acceptance that completes after token resolution but before the lock' do
    token = invitation.plain_token
    accept(token)
    expect(response).to have_http_status(:ok)
    allow(HouseholdInvitations::TokenResolver).to receive(:call).with(token).and_return(invitation.reload)
    accept(token)
    expect(response).to have_http_status(:ok)
    expect(target.household_memberships.where(account: account).count).to eq(1)
  end

  it 'rejects a token replaced by resend after token resolution' do
    token = invitation.plain_token
    invitation.resend!
    allow(HouseholdInvitations::TokenResolver).to receive(:call).with(token).and_return(invitation)
    accept(token)
    expect(response).to have_http_status(:unprocessable_content)
    expect(target.household_memberships.where(account: account)).to be_empty
  end

  it 'rechecks verified account state after resolving the token' do
    allow(HouseholdInvitations::TokenResolver).to receive(:call).and_wrap_original do |original, token|
      original.call(token).tap { account.update!(status: :closed) }
    end
    accept
    expect(response).to have_http_status(:unprocessable_content)
    expect(target.household_memberships.where(account: account)).to be_empty
  end

  it 'rolls back the person, membership and grants when delegated grant creation fails' do
    patient = target.people.create!(name: 'Invited Patient', date_of_birth: 40.years.ago.to_date)
    invitation.household_invitation_grants.create!(household: target, person: patient, access_level: :record,
                                                   relationship_type: :professional)
    service = instance_double(CareDelegation::Assign)
    allow(CareDelegation::Assign).to receive(:new).and_return(service)
    allow(service).to receive(:call).and_raise(CareDelegation::Assign::GrantConflict)
    expect { accept }.not_to change(Person, :count)
    expect(response).to have_http_status(:unprocessable_content)
    expect(target.household_memberships.where(account: account)).to be_empty
    expect(invitation.reload.accepted_at).to be_nil
  end
end
