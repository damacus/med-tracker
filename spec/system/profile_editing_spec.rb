# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile Editing' do
  fixtures :accounts, :account_otp_keys, :people, :users

  let(:account) { accounts(:damacus) }
  let(:person) { people(:damacus) }
  let(:user) { users(:damacus) }

  before do
    login_as(user)
    visit profile_path
  end

  after do |example|
    # Clean up any inserted modal content between JS tests
    page.execute_script('document.querySelectorAll("[data-state]").forEach(el => el.remove())') if example.metadata[:js]
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
      first('button', text: 'Change').click

      # Wait for Stimulus to insert sheet content into DOM
      expect(page).to have_field('your.email@example.com', with: account.email)
    end

    it 'updates email when saving' do
      expect(page).to have_css('[data-ruby-ui--sheet-target="content"]', visible: :hidden, wait: 5)

      first('button', text: 'Change').click

      expect(page).to have_field('your.email@example.com', with: account.email, wait: 10)

      fill_in 'your.email@example.com', with: 'newemail@example.com'
      find('input[name="account[email]"]').send_keys(:enter)

      expect(page).to have_content('Email updated successfully')
      expect(account.reload.email).to eq('newemail@example.com')
    end
  end

  describe 'changing password', :js do
    it 'opens sheet modal when clicking change' do
      expect(page).to have_css('[data-ruby-ui--sheet-target="content"]', visible: :hidden, wait: 5)

      # Click the second Change button (for password)
      all('button', text: 'Change')[1].click

      # Wait for sheet content to appear
      expect(page).to have_field('Enter current password', wait: 10)
      expect(page).to have_field('Enter new password')
      expect(page).to have_field('Confirm new password')
    end
  end

  describe 'closing account', :js do
    it 'shows confirmation dialog when clicking close account' do
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_button 'Close Account'

      # Wait for AlertDialog content to appear
      expect(page).to have_content('Are you absolutely sure?', wait: 10)
      expect(page).to have_content('This action cannot be undone')
      expect(page).to have_button('Cancel')
      expect(page).to have_button('Yes, delete my account')
    end

    it 'can cancel account closure' do
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_button 'Close Account'
      expect(page).to have_content('Are you absolutely sure?', wait: 10)

      click_button 'Cancel'

      # Dialog should close and we're still on profile page
      expect(page).to have_content('My Profile')
    end
  end
end
