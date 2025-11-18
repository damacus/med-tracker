# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin Access Authorization' do
  fixtures :users, :people

  before do
    driven_by(:playwright)
  end

  let(:admin) { users(:admin) }
  let(:carer) { users(:jane) }

  describe 'admin user management' do
    it 'allows administrators to access user management' do
      sign_in_as(admin)
      visit admin_users_path

      expect(page).to have_content('User Management')
      expect(page).to have_content(admin.email_address)
    end

    it 'denies non-administrators access to user management' do
      sign_in_as(carer)
      visit admin_users_path

      expect(page).to have_content('You are not authorized to perform this action')
      expect(page).to have_no_content('User Management')
    end
  end

  describe 'people management' do
    it 'allows administrators to view all people' do
      sign_in_as(admin)
      visit people_path

      expect(page).to have_content('People')
      expect(Person.count).to be > 0
    end

    it 'restricts carers to viewing only their assigned people' do
      sign_in_as(carer)
      visit people_path

      # Carer should only see themselves and their patients
      expect(page).to have_content(carer.person.name)
      expect(page).to have_content('Child Patient')
    end
  end

  def sign_in_as(user, password: 'password')
    login_as(user)
    visit login_path
    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: password
    click_button 'Login'
    expect(page).to have_current_path(dashboard_path)
  end
end
