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
