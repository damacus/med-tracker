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

  describe 'changing email', :js do
    it 'opens modal when clicking change' do
      click_on 'Change', match: :prefer_exact

      expect(page).to have_css('dialog[open]')
      expect(page).to have_content('Change Login')
    end

    it 'submits email change request when saving' do
      click_on 'Change', match: :prefer_exact

      expect(page).to have_css('dialog[open]')

      fill_in 'New Login', with: 'newemail@example.com'
      fill_in 'Password', with: 'password'
      click_on 'Change Login'

      expect(page).to have_content('An email has been sent to you with a link to verify your login change')
    end
  end

  describe 'changing password', :js do
    it 'opens modal when clicking change' do
      # In M3, these are likely m3_links which are anchors
      all('a', text: 'Change')[1].click

      expect(page).to have_css('dialog[open]')
      expect(page).to have_content('Change Password')
    end
  end

  describe 'closing account', :js do
    it 'shows confirmation dialog when clicking close account' do
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_on 'Close Account'

      # Wait for AlertDialog content to appear
      expect(page).to have_content('Are you absolutely sure?', wait: 10)
      expect(page).to have_content('This action cannot be undone')
      expect(page).to have_button('Cancel')
      expect(page).to have_content('Yes, delete my account')
    end

    it 'can cancel account closure' do
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_on 'Close Account'
      expect(page).to have_content('Are you absolutely sure?', wait: 10)

      click_on 'Cancel'

      # Dialog should close and we're still on profile page
      expect(page).to have_content('My Profile')
    end

    it 'closes the account and prevents future login' do
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_on 'Close Account'
      expect(page).to have_content('Are you absolutely sure?', wait: 10)

      fill_in 'Password', with: 'password'
      click_on 'Yes, delete my account'

      expect(page).to have_current_path('/login')
      expect(account.reload).to be_closed
      expect(person.reload.account).to be_nil

      fill_in 'Email address', with: account.email
      fill_in 'Password', with: 'password'
      click_on 'Sign In'

      expect(page).to have_no_current_path('/dashboard')
      expect(page).to have_content(/closed|invalid|error/i)
    end
  end
end
