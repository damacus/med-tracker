require 'rails_helper'

RSpec.describe 'API v1 invitation resend' do
  fixtures :all

  let(:login) { api_login(users(:admin)) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:session) { ApiSession.lookup_by_access_token(login.fetch('access_token')) }
  let(:invitation) do
    session.household_membership.household.household_invitations.create!(
      email: 'resend@example.test', membership_role: :member, invited_by_membership: session.household_membership
    )
  end
  let(:path) { "/api/v1/households/#{login.dig('household', 'id')}/admin/invitations/#{invitation.id}/resend" }

  before do
    session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
    invitation
  end

  around do |example|
    previous_adapter = ActiveJob::Base.queue_adapter
    ActiveJob::Base.queue_adapter = :test
    example.run
  ensure
    ActiveJob::Base.queue_adapter = previous_adapter
  end

  it 'queues the existing invitation mail and invalidates the old token' do
    old_token = invitation.plain_token
    expect { post path, headers: headers, as: :json }.to have_enqueued_mail(InvitationMailer, :invite)
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.fetch('data')).to include('invitation_id' => invitation.id.to_s,
                                                          'delivery_status' => 'queued')
    expect(invitation.reload.token_digest).not_to eq(HouseholdInvitation.digest(old_token))
    expect(HouseholdInvitations::TokenResolver.call(old_token)).to be_nil
    expect(response.body).not_to include(old_token, invitation.token_digest)
    expect(SecurityAuditEvent.where(event_type: 'api/admin/invitation/resent').count).to eq(1)
  end

  it 'requires fresh MFA before replaying a successful response' do
    replay_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    post path, headers: replay_headers, as: :json
    expect(response).to have_http_status(:ok)
    digest = invitation.reload.token_digest
    session.update!(mfa_verified_at: 20.minutes.ago)
    expect { post path, headers: replay_headers, as: :json }.not_to have_enqueued_mail(InvitationMailer, :invite)
    expect(response).to have_http_status(:forbidden)
    expect(response.parsed_body.dig('error', 'code')).to eq('fresh_privileged_action_required')
    expect(invitation.reload.token_digest).to eq(digest)
  end

  it 'checks current administrator authority before replaying a successful response' do
    replay_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    post path, headers: replay_headers, as: :json
    expect(response).to have_http_status(:ok)
    other_owner = Account.create!(email: 'resend-owner@example.test', status: :verified)
    invitation.household.household_memberships.create!(account: other_owner, role: :owner, status: :active)
    session.household_membership.update!(role: :member)
    post path, headers: replay_headers, as: :json
    expect(response).to have_http_status(:forbidden)
  end

  it 'renews an expired invitation but rejects accepted and revoked invitations' do
    invitation.update!(expires_at: 1.day.ago)
    post path, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(invitation.reload.expires_at).to be_future
    invitation.update!(accepted_at: Time.current)
    expect { post path, headers: headers, as: :json }.not_to have_enqueued_mail(InvitationMailer, :invite)
    expect(response).to have_http_status(:unprocessable_content)
    invitation.update!(accepted_at: nil, revoked_at: Time.current)
    post path, headers: headers, as: :json
    expect(response).to have_http_status(:unprocessable_content)
  end

  it 'retains the old token and rolls back the audit if mail enqueueing fails' do
    digest = invitation.token_digest
    allow(Observability::EmergencyDiagnostic).to receive(:write).and_call_original
    allow(InvitationMailer).to receive(:with).and_raise(IOError, 'Private mail credentials')
    post path, headers: headers, as: :json
    expect(response).to have_http_status(:service_unavailable)
    expect(invitation.reload.token_digest).to eq(digest)
    expect(response.body).not_to include('Private mail credentials')
    expect(Observability::EmergencyDiagnostic).not_to have_received(:write)
    expect(SecurityAuditEvent.where(event_type: 'api/admin/invitation/resent')).to be_empty
  end

  it 'rechecks credential revocation before rotating the invitation token' do
    digest = invitation.token_digest
    allow(HouseholdInvitations::Resend).to receive(:new).and_wrap_original do |original, **options|
      session.update!(revoked_at: Time.current)
      original.call(**options)
    end
    expect { post path, headers: headers, as: :json }.not_to have_enqueued_mail(InvitationMailer, :invite)
    expect(response).to have_http_status(:forbidden)
    expect(invitation.reload.token_digest).to eq(digest)
  end

  it 'works under the application database role and does not queue duplicate cached retries' do
    replay_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    endpoint = path
    ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
    post endpoint, headers: replay_headers, as: :json
    expect(response).to have_http_status(:ok)
    expect { post endpoint, headers: replay_headers, as: :json }.not_to have_enqueued_mail(InvitationMailer, :invite)
    expect(response).to have_http_status(:ok)
    expect(response.headers['Cache-Control']).to include('no-store')
  end

  it 'advertises resend as an online invitation action' do
    get '/api/v1/capabilities', as: :json
    expect(response.parsed_body.dig('data', 'invitations', 'actions')).to eq(%w[accept resend])
  end
end
