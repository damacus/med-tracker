# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Minor role in user form' do
  fixtures :users

  let(:admin) { users(:admin) }

  before do
    driven_by(:rack_test)
  end

  it 'shows minor role in the dropdown' do
    sign_in_as(admin)

    visit new_admin_user_path

    expect(page).to have_select('Role', with_options: ['Minor'])
  end

  it 'allows creating a user with minor role' do
    sign_in_as(admin)

    visit new_admin_user_path

    fill_in 'Name', with: 'Minor User'
    fill_in 'Date of birth', with: '2015-11-04'
    fill_in 'Email address', with: 'minor@example.com'
    fill_in 'Password', with: 'password123'
    fill_in 'Password confirmation', with: 'password123'
    select 'Minor', from: 'Role'

    click_button 'Create User'

    expect(page).to have_content('User was successfully created')
    expect(page).to have_content('minor@example.com')
    expect(page).to have_content('minor')
  end

  def sign_in_as(user, password: 'password')
    login_as(user)
    login_as(user)
  end
end
