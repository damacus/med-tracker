# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin settings' do
  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }

  before do
    PlatformAdmin.create!(account: admin.person.account)
    sign_in(admin)
  end

  describe 'GET /admin/settings' do
    it 'renders household admin settings' do
      get admin_settings_path

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Settings')
    end
  end

  describe 'PATCH /admin/settings' do
    it 'updates invite-only mode for household admins' do
      AppSettings.instance.update!(invite_only: true)

      patch admin_settings_path, params: { app_settings: { invite_only: '0' } }

      expect(response).to redirect_to(admin_settings_path)
      expect(AppSettings.instance.reload.invite_only).to be(false)
    end

    it 'returns turbo streams for successful updates' do
      patch admin_settings_path,
            params: { app_settings: { invite_only: '0' } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="admin_settings"')
      expect(response.body).to include('target="flash"')
    end

    it 'renders validation errors when settings cannot be updated' do
      settings = AppSettings.instance
      allow(AppSettings).to receive(:instance).and_return(settings)
      allow(settings).to receive(:update).and_return(false)

      patch admin_settings_path, params: { app_settings: { invite_only: '0' } }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('Settings')
    end
  end
end
