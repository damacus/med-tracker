# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin create and update turbo flows' do
  fixtures :accounts, :people, :users, :locations, :carer_relationships

  let(:admin) { users(:admin) }

  before { sign_in(admin) }

  describe 'POST /admin/invitations' do
    it 'returns turbo_stream and updates invitations container and flash on success' do
      post admin_invitations_path,
           params: { invitation: { email: 'new.user@example.com', role: 'carer' } },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="admin_invitations"')
      expect(response.body).to include('target="flash"')
    end

    it 'returns unprocessable content when inviting a minor' do
      post admin_invitations_path,
           params: { invitation: { email: 'minor.user@example.com', role: 'minor' } },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('Children must be added by a parent or carer.')
    end
  end

  describe 'POST /admin/invitations/:id/resend' do
    let!(:invitation) { create(:invitation, email: 'existing.user@example.com', role: :carer, expires_at: 1.day.ago) }

    it 'returns turbo_stream, rotates the token, records an audit event, and updates flash on success' do
      original_token = invitation.token

      post resend_admin_invitation_path(invitation),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="admin_invitations"')
      expect(response.body).to include('target="flash"')
      expect(invitation.reload.token).not_to eq(original_token)
      expect(invitation.versions.last.event).to eq('resend')
      expect(invitation.versions.last.whodunnit).to eq(admin.id.to_s)
    end

    it 'returns an alert and leaves accepted invitations unchanged' do
      accepted_invitation = create(:invitation, :accepted, email: 'accepted.user@example.com', role: :carer)

      post resend_admin_invitation_path(accepted_invitation),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('Accepted invitations cannot be resent')
      expect(response.body).not_to include('Invitation resent')
      expect(accepted_invitation.reload.versions.last.event).not_to eq('resend')
    end

    it 'returns an alert and leaves the invitation unchanged when a newer pending invitation exists' do
      conflicting_invitation = create(:invitation, :expired, email: 'duplicate.user@example.com', role: :carer)
      create(:invitation, email: 'duplicate.user@example.com', role: :carer)
      original_token = conflicting_invitation.token
      original_expires_at = conflicting_invitation.expires_at

      post resend_admin_invitation_path(conflicting_invitation),
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include(
        'This invitation could not be resent. A pending invitation for this email may already exist.'
      )
      conflicting_invitation.reload
      expect(conflicting_invitation.token).to eq(original_token)
      expect(conflicting_invitation.expires_at.to_i).to eq(original_expires_at.to_i)
    end
  end

  describe 'POST /admin/users' do
    it 'follows turbo redirect to index on success' do
      post admin_users_path,
           params: {
             user: {
               email_address: 'turbo.new.user@example.com',
               password: 'password',
               password_confirmation: 'password',
               role: 'carer',
               person_attributes: {
                 name: 'Turbo New User',
                 date_of_birth: '1990-01-01',
                 location_ids: [locations(:home).id]
               }
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:redirect)
      expect(response).to redirect_to(admin_users_path)
    end

    it 'returns unprocessable content on validation failure' do
      post admin_users_path,
           params: {
             user: {
               email_address: users(:jane).email_address,
               password: 'password',
               password_confirmation: 'password',
               role: 'carer',
               person_attributes: {
                 name: 'Duplicate Email User',
                 date_of_birth: '1990-01-01',
                 location_ids: [locations(:home).id]
               }
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('role="alert"')
      expect(response.body).to include('id="user_email_address_error"')
      expect(response.body).to include('aria-describedby="user_email_address_error"')
      expect(response.body).to include('aria-invalid')
    end
  end

  describe 'PATCH /admin/users/:id' do
    it 'follows turbo redirect to index on success' do
      patch admin_user_path(users(:jane)),
            params: {
              user: {
                email_address: users(:jane).email_address,
                role: 'parent',
                person_attributes: {
                  id: users(:jane).person.id,
                  name: 'Jane Turbo Update',
                  date_of_birth: users(:jane).person.date_of_birth.to_s,
                  location_ids: [locations(:home).id]
                }
              }
            },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:redirect)
      expect(response).to redirect_to(admin_users_path)
      expect(users(:jane).person.reload.name).to eq('Jane Turbo Update')
    end
  end

  describe 'POST /admin/carer_relationships' do
    it 'returns turbo_stream and updates modal, rows, and flash on success' do
      post admin_carer_relationships_path,
           params: {
             carer_relationship: {
               carer_id: people(:jane).id,
               patient_id: people(:john).id,
               relationship_type: 'family_member'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('target="carer_relationships_rows"')
      expect(response.body).to include('target="flash"')
    end
  end

  describe 'GET /admin/carer_relationships/new' do
    it 'returns turbo_stream and replaces modal frame' do
      get new_admin_carer_relationship_path,
          headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('carer_relationship_carer_id')
    end
  end
end
