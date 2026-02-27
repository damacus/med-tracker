# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile Editing' do
  fixtures :accounts, :people, :users

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

    it 'shows account security section with change links' do
      expect(page).to have_content('Account Security')
      expect(page).to have_content('Change Email Address')
      expect(page).to have_content('Change Password')
      expect(page).to have_link('Change', count: 2)
    end

    it 'shows danger zone with close account button' do
      expect(page).to have_content('Danger Zone')
      expect(page).to have_content('Close Account')
      expect(page).to have_button('Close Account')
    end

    it 'has modal frames for account actions' do
      # Check if links target the modal frame
      expect(page).to have_css("a[data-turbo-frame='modal']", count: 2)
    end

    it 'has AlertDialog for close account' do
      # Check if AlertDialog component exists
      expect(page).to have_css('[data-controller="ruby-ui--alert-dialog"]', minimum: 1)
    end
  end

  describe 'changing email', :js do
    it 'opens modal when clicking change' do
      first('a', text: 'Change').click

      expect(page).to have_css('dialog[open]')
      expect(page).to have_content('Change Login')
    end

    it 'submits email change request when saving' do
      first('a', text: 'Change').click

      expect(page).to have_css('dialog[open]')

      fill_in 'New Login', with: 'newemail@example.com'
      fill_in 'Password', with: 'password'
      click_button 'Change Login'

      expect(page).to have_content('An email has been sent to you with a link to verify your login change')
    end
  end

  describe 'changing password', :js do
    it 'opens modal when clicking change' do
      all('a', text: 'Change')[1].click

      expect(page).to have_css('dialog[open]')
      expect(page).to have_content('Change Password')
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
