# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin turbo actions' do
  fixtures :accounts, :people, :users, :carer_relationships

  let(:admin) { users(:admin) }

  before { sign_in(admin) }

  describe 'POST /admin/users/:id/activate' do
    it 'returns turbo_stream and updates the user row and flash' do
      user = users(:jane)
      user.deactivate!

      post activate_admin_user_path(user), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"user_#{user.id}\"")
      expect(response.body).to include('target="flash"')
      expect(user.reload).to be_active
    end
  end

  describe 'DELETE /admin/users/:id' do
    it 'returns turbo_stream and updates the user row and flash' do
      user = users(:jane)

      delete admin_user_path(user), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"user_#{user.id}\"")
      expect(response.body).to include('target="flash"')
      expect(user.reload).not_to be_active
    end
  end

  describe 'POST /admin/users/:id/verify' do
    it 'returns turbo_stream and updates the user row and flash' do
      user = users(:jane)
      user.person.account.update!(status: :unverified)

      post verify_admin_user_path(user), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"user_#{user.id}\"")
      expect(response.body).to include('target="flash"')
      expect(user.person.account.reload).to be_verified
    end
  end

  describe 'POST /admin/carer_relationships/:id/activate' do
    it 'returns turbo_stream and updates the relationship row and flash' do
      relationship = carer_relationships(:inactive_relationship)

      post activate_admin_carer_relationship_path(relationship), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"carer_relationship_#{relationship.id}\"")
      expect(response.body).to include('target="flash"')
      expect(relationship.reload).to be_active
    end
  end

  describe 'DELETE /admin/carer_relationships/:id' do
    it 'returns turbo_stream and updates the relationship row and flash' do
      relationship = carer_relationships(:jane_cares_for_child)

      delete admin_carer_relationship_path(relationship), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"carer_relationship_#{relationship.id}\"")
      expect(response.body).to include('target="flash"')
      expect(relationship.reload).not_to be_active
    end
  end
end
