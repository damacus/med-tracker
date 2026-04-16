# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Minor role in user form' do
  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }

  before do
    driven_by(:playwright)
  end

  it 'shows minor role in the dropdown' do
    login_as(admin)

    visit new_admin_user_path

    find('#role_trigger').click
    expect(page).to have_content('Minor')
  end

  it 'allows creating a user with minor role' do
    login_as(admin)

    visit new_admin_user_path

    fill_in 'Name', with: 'Minor User'
    fill_in 'Date of birth', with: '2015-11-04'
    fill_in 'Email address', with: 'minor@example.com'
    fill_in 'user_password', with: 'password123'
    fill_in 'user_password_confirmation', with: 'password123'
    
    find('#role_trigger').click
    all('label', text: 'Minor', visible: :all).last.click

    click_on 'Create User'

    expect(page).to have_content('User was successfully created')
    expect(page).to have_content('minor@example.com')
    expect(page).to have_content('Minor')
  end
end
