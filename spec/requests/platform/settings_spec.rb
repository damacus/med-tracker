# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Platform settings' do
  fixtures :all

  let(:platform_user) { users(:admin) }
  let(:household_owner) { users(:damacus) }

  it 'allows an active platform admin outside household routes' do
    PlatformAdmin.create!(account: platform_user.person.account)
    sign_in(platform_user)

    get platform_settings_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Platform Settings')
  end

  it 'updates platform settings for an active platform admin' do
    PlatformAdmin.create!(account: platform_user.person.account)
    sign_in(platform_user)

    patch platform_settings_path, params: { app_settings: { invite_only: '0' } }

    expect(response).to redirect_to(platform_settings_path)
    expect(AppSettings.instance.reload.invite_only).to be(false)
  end

  it 'renders validation errors when platform settings cannot be updated' do
    PlatformAdmin.create!(account: platform_user.person.account)
    settings = AppSettings.instance
    allow(AppSettings).to receive(:instance).and_return(settings)
    allow(settings).to receive(:update).and_return(false)
    sign_in(platform_user)

    patch platform_settings_path, params: { app_settings: { invite_only: '0' } }

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).to include('Platform Settings')
  end

  it 'denies a household owner without platform admin access' do
    sign_in(household_owner)

    get platform_settings_path

    expect(response).to redirect_to(root_path)
  end

  it 'keeps household admin settings denied to household managers without platform admin access' do
    sign_in(household_owner)

    get admin_settings_path

    expect(response).to redirect_to(root_path)
  end
end
