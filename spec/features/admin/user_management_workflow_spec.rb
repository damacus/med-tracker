# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin User Management Workflow', type: :system do
  fixtures :all

  let(:admin) { users(:damacus) }

  before do
    driven_by(:playwright)
  end

  it 'completes full user lifecycle from creation to activation' do
    # Step 1: Log in as administrator (damacus@example.com)
    sign_in(admin)

    # Step 2: Navigate to /admin/users
    visit admin_users_path

    # Step 3: Verify user list displayed with columns
    within '[data-testid="admin-users"]' do
      expect(page).to have_content('User Management')
      expect(page).to have_content('Name')
      expect(page).to have_content('Email')
      expect(page).to have_content('Role')
      expect(page).to have_content('Status')
      expect(page).to have_content('Actions')
    end

    # Step 4: Click New User button
    click_link 'New User'

    # Verify we're on the new user form
    expect(page).to have_content('Create New User')

    # Step 5-9: Fill in the form
    fill_in 'Name', with: 'New Test User'
    fill_in 'Date of birth', with: '1990-01-15'
    fill_in 'Email address', with: 'newuser@example.com'
    fill_in 'Password', with: 'SecurePass123'
    fill_in 'Password confirmation', with: 'SecurePass123'
    select 'Nurse', from: 'Role'

    # Step 10: Submit form
    click_button 'Create User'

    # Step 11: Verify success message
    expect(page).to have_content('User was successfully created')

    # Step 12: Verify user appears in list
    within '[data-testid="admin-users"]' do
      expect(page).to have_content('newuser@example.com')
      expect(page).to have_content('New Test User')
      expect(page).to have_content('Nurse')
    end

    # Find the new user's row by email to interact with it
    new_user = User.find_by(email_address: 'newuser@example.com')

    # Step 13: Click Edit on new user
    within "[data-user-id='#{new_user.id}']" do
      click_link 'Edit'
    end

    # Verify we're on the edit page
    expect(page).to have_content('Edit User')
    expect(page).to have_field('Email address', with: 'newuser@example.com')

    # Step 14: Change role to doctor
    select 'Doctor', from: 'Role'

    # Step 15: Submit changes
    click_button 'Update User'

    # Step 16: Verify role updated
    expect(page).to have_content('User was successfully updated')
    within "[data-user-id='#{new_user.id}']" do
      expect(page).to have_content('Doctor')
    end

    # Step 17: Click Deactivate
    within "[data-user-id='#{new_user.id}']" do
      expect(page).to have_content('Active')
      click_button 'Deactivate'
    end

    # Step 18: Confirm deactivation
    within('[role="alertdialog"]') do
      expect(page).to have_content('Deactivate User Account')
      expect(page).to have_content("Are you sure you want to deactivate New Test User's account?")
      click_button 'Deactivate'
    end

    # Step 19: Verify user marked inactive
    expect(page).to have_content('User account has been deactivated')
    within "[data-user-id='#{new_user.id}']" do
      expect(page).to have_content('Inactive')
    end

    # Step 20: Click Activate
    within "[data-user-id='#{new_user.id}']" do
      click_button 'Activate'
    end

    # Step 21: Verify user marked active again
    expect(page).to have_content('User account has been activated')
    within "[data-user-id='#{new_user.id}']" do
      expect(page).to have_content('Active')
    end
  end
end
