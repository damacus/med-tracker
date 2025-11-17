# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile Editing' do
  fixtures :accounts, :people, :users

  let(:account) { accounts(:damacus) }
  let(:person) { people(:damacus) }
  let(:user) { users(:damacus) }

  before do
    rodauth_login(account.email, 'password')
    visit profile_path
  end

  describe 'viewing profile' do
    it 'displays user information' do
      expect(page).to have_content('My Profile')
      expect(page).to have_content(person.name)
      expect(page).to have_content(account.email)
    end

    it 'shows account security section with change buttons' do
      expect(page).to have_content('Account Security')
      expect(page).to have_content('Change Email Address')
      expect(page).to have_content('Change Password')
      expect(page).to have_button('Change', count: 2)
    end

    it 'shows danger zone with close account button' do
      expect(page).to have_content('Danger Zone')
      expect(page).to have_content('Close Account')
      expect(page).to have_button('Close Account')
    end

    it 'has Stimulus controller attributes on sheet elements' do
      # Check if Sheet components exist for email/password changes
      expect(page).to have_css('[data-controller="ruby-ui--sheet"]', minimum: 2)
      expect(page).to have_css('[data-action*="ruby-ui--sheet#open"]', minimum: 2)
    end

    it 'has AlertDialog for close account' do
      # Check if AlertDialog component exists
      expect(page).to have_css('[data-controller="ruby-ui--alert-dialog"]', minimum: 1)
    end
  end

  describe 'changing email', :js do
    it 'opens sheet modal when clicking change' do
      # Click the first Change button (for email)
      first('button', text: 'Change').click

      # Wait for animation and sheet to be visible
      expect(page).to have_content('Change Email Address', wait: 2)
      expect(page).to have_field('your.email@example.com', with: account.email)
    end

    it 'updates email when saving' do
      first('button', text: 'Change').click

      # Wait for sheet to be visible
      expect(page).to have_content('Change Email Address', wait: 2)

      fill_in 'your.email@example.com', with: 'newemail@example.com'
      click_button 'Save'

      expect(page).to have_content('Email updated successfully')
      expect(account.reload.email).to eq('newemail@example.com')
    end
  end

  describe 'changing password', :js do
    it 'opens sheet modal when clicking change' do
      # Click the second Change button (for password)
      all('button', text: 'Change')[1].click

      # Wait for animation and sheet to be visible
      expect(page).to have_content('Change Password', wait: 2)
      expect(page).to have_field('Enter current password')
      expect(page).to have_field('Enter new password')
      expect(page).to have_field('Confirm new password')
    end
  end

  describe 'closing account', :js do
    it 'shows confirmation dialog when clicking close account' do
      click_button 'Close Account'

      # Wait for AlertDialog to appear
      expect(page).to have_content('Are you absolutely sure?', wait: 2)
      expect(page).to have_content('This action cannot be undone')
      expect(page).to have_button('Cancel')
      expect(page).to have_button('Yes, delete my account')
    end

    it 'can cancel account closure' do
      click_button 'Close Account'
      expect(page).to have_content('Are you absolutely sure?', wait: 2)

      click_button 'Cancel'

      # Dialog should close and we're still on profile page
      expect(page).to have_content('My Profile')
    end
  end
end
