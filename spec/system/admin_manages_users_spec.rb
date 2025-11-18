# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AdminManagesUsers' do
  fixtures :users

  # Use the admin fixture instead of creating a duplicate user
  let(:admin) { users(:admin) }
  # Use a unique email for the carer user to avoid conflicts
  let!(:carer) do
    account = Account.create!(email: 'test_carer@example.com',
                              password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
                              status: 'verified')
    person = Person.create!(name: 'Carer User', date_of_birth: '1990-01-01', account: account)
    User.create!(person: person, email_address: 'test_carer@example.com',
                 password: 'password', password_confirmation: 'password', role: :carer)
  end

  before do
    driven_by(:rack_test)
  end

  context 'when user is logged in as an admin' do
    it 'allows admin to see the list of users' do
      sign_in_as(admin)

      visit admin_users_path

      within '[data-testid="admin-users"]' do
        expect(page).to have_content('User Management')
        expect(page).to have_content(admin.email_address)
        expect(page).to have_content(carer.email_address)
      end
    end

    it 'allows admin to create a new user' do
      sign_in_as(admin)

      visit admin_users_path
      click_link 'New User'

      expect(page).to have_content('Create New User')

      fill_in 'Name', with: 'New Test User'
      fill_in 'Date of birth', with: '1985-05-15'
      fill_in 'Email address', with: 'newuser@example.com'
      fill_in 'Password', with: 'password123'
      fill_in 'Password confirmation', with: 'password123'
      select 'Doctor', from: 'Role'

      click_button 'Create User'

      expect(page).to have_content('User was successfully created')
      expect(page).to have_content('newuser@example.com')
      expect(page).to have_content('doctor')
    end

    it 'shows validation errors when creating user with invalid data' do
      sign_in_as(admin)

      visit new_admin_user_path

      click_button 'Create User'

      expect(page).to have_content('Email address can\'t be blank')
    end

    it 'allows admin to edit an existing user' do
      sign_in_as(admin)

      visit admin_users_path
      within "[data-user-id='#{carer.id}']" do
        click_link 'Edit'
      end

      expect(page).to have_content('Edit User')
      expect(page).to have_field('Email address', with: carer.email_address)

      fill_in 'Email address', with: 'updated_carer@example.com'
      select 'Nurse', from: 'Role'

      click_button 'Update User'

      expect(page).to have_content('User was successfully updated')
      expect(page).to have_content('updated_carer@example.com')
      expect(page).to have_content('nurse')
    end

    it 'shows validation errors when updating user with invalid data' do
      sign_in_as(admin)

      visit edit_admin_user_path(carer)

      fill_in 'Email address', with: ''

      click_button 'Update User'

      expect(page).to have_content('Email address can\'t be blank')
    end

    it 'allows admin to search users by name' do
      sign_in_as(admin)

      visit admin_users_path

      fill_in 'Search', with: 'Carer'
      click_button 'Search'

      within '[data-testid="admin-users"]' do
        expect(page).to have_content('Carer User')
        expect(page).to have_no_content(admin.name)
      end
    end

    it 'allows admin to search users by email' do
      sign_in_as(admin)

      visit admin_users_path

      fill_in 'Search', with: 'carer@example.com'
      click_button 'Search'

      expect(page).to have_content('carer@example.com')
      expect(page).to have_no_content(admin.email_address)
    end

    it 'allows admin to filter users by role' do
      sign_in_as(admin)

      visit admin_users_path

      select 'Carer', from: 'Role'
      click_button 'Search'

      expect(page).to have_content('test_carer@example.com')
      expect(page).to have_no_content(admin.email_address)
    end

    it 'allows admin to combine search and filter' do
      sign_in_as(admin)

      visit admin_users_path

      fill_in 'Search', with: 'Carer'
      select 'Carer', from: 'Role'
      click_button 'Search'

      within '[data-testid="admin-users"]' do
        expect(page).to have_content('Carer User')
        expect(page).to have_no_content(admin.name)
      end
    end

    it 'shows all users when search is cleared' do
      sign_in_as(admin)

      visit admin_users_path

      fill_in 'Search', with: 'Carer'
      click_button 'Search'

      click_link 'Clear'

      expect(page).to have_content('Carer User')
      expect(page).to have_content(admin.name)
    end
  end

  context 'when user is logged in as a non-admin' do
    it 'denies access to the user list' do
      sign_in_as(carer)

      visit admin_users_path

      expect(page).to have_css('#flash', text: 'You are not authorized to perform this action.')
    end

    it 'denies access to create new users' do
      sign_in_as(carer)

      visit new_admin_user_path

      expect(page).to have_css('#flash', text: 'You are not authorized to perform this action.')
    end
  end

  def sign_in_as(user, password: 'password')
    visit login_path
    fill_in 'Email address', with: user.email_address
    fill_in 'Password', with: password
    click_button 'Login'
    expect(page).to have_current_path(dashboard_path)
  end
end
