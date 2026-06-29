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
