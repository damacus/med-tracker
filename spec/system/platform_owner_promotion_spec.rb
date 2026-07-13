# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Platform owner promotion' do
  fixtures :all

  it 'shows the owner promotion action to a platform administrator' do
    platform_user = users(:admin)
    platform_admin = PlatformAdmin.find_or_create_by!(account: platform_user.person.account)
    platform_admin.active!

    login_as(platform_user)
    visit platform_users_path

    expect(page).to have_button('Promote to owner')
  end
end
