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

      expect(page).to have_css('[data-controller="ruby-ui--dialog"] [data-state="open"]')
      expect(page).to have_text('Change Login')
    end

    it 'submits email change request when saving' do
      click_on 'Change', match: :prefer_exact

      expect(page).to have_css('[data-controller="ruby-ui--dialog"] [data-state="open"]')

      fill_in 'New Login', with: 'newemail@example.com'
      fill_in 'Password', with: 'password'
      click_on 'Change Login'

      expect(page).to have_text('An email has been sent to you with a link to verify your login change')
    end
  end

  describe 'changing password', :js do
    it 'opens modal when clicking change' do
      # In M3, these are likely m3_links which are anchors
      all('a', text: 'Change')[1].click

      expect(page).to have_css('[data-controller="ruby-ui--dialog"] [data-state="open"]')
      expect(page).to have_text('Change Password')
    end
  end

  describe 'time zone preference', :js do
    it 'keeps a stored non-default time zone when saved without a change' do
      stored_time_zone = 'Europe/Belfast'
      account.preferences = account.preferences.merge('time_zone' => stored_time_zone)
      account.save!(validate: false)
      account.reload

      visit profile_path

      time_zone_row = find('dt', text: 'Time Zone').find(:xpath, 'following-sibling::dd')
      expect(time_zone_row).to have_text(stored_time_zone)
      expect(page).to have_select('Time Zone', selected: stored_time_zone)

      click_button 'Save time zone'

      expect(page).to have_text(I18n.t('profiles.updated'))
      expect(account.reload.time_zone).to eq(stored_time_zone)
    end

    it 'saves a deliberately selected time zone' do
      visit profile_path

      select 'Pacific Time (US & Canada)', from: 'Time Zone'
      click_button 'Save time zone'

      expect(page).to have_text(I18n.t('profiles.updated'))
      expect(account.reload.time_zone).to eq('Pacific Time (US & Canada)')
    end
  end

  describe 'closing account', :js do
    it 'shows confirmation dialog when clicking close account' do
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_on 'Close Account'

      # Wait for AlertDialog content to appear
      expect(page).to have_text('Are you absolutely sure?', wait: 10)
      expect(page).to have_text('This action cannot be undone')
      expect(page).to have_button('Cancel')
      expect(page).to have_text('Yes, delete my account')
    end

    it 'keeps the close-account confirmation full width on desktop' do
      page.current_window.resize_to(1400, 1000)
      click_on 'Close Account'

      dialog = find('dialog[open][role="alertdialog"]')
      widths = dialog.evaluate_script(<<~JS)
        ({
          password: this.querySelector('#close-account-password').getBoundingClientRect().width,
          confirm: Array.from(this.querySelectorAll('button')).find((button) =>
            button.textContent.includes('Yes, delete my account')
          ).getBoundingClientRect().width
        })
      JS
      expect(widths.fetch('confirm')).to be_within(1).of(widths.fetch('password'))
    end

    it 'can cancel account closure' do
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_on 'Close Account'
      expect(page).to have_text('Are you absolutely sure?', wait: 10)

      click_on 'Cancel'

      # Dialog should close and we're still on profile page
      expect(page).to have_text('My Profile')
    end

    it 'keeps keyboard focus inside the account closure alert dialog' do
      click_on 'Close Account'

      dialog = find('dialog[open][role="alertdialog"]')
      tabbable_count = dialog.all(
        'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), ' \
        'select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      ).count

      expect(tabbable_count).to be_positive
      expect(page).to have_css('[role="alertdialog"] :focus')
      expect(page.evaluate_script("document.activeElement.closest('[role=alertdialog]') !== null")).to be(true)

      (tabbable_count + 1).times do
        page.active_element.send_keys(:tab)
        active_element_in_dialog = page.evaluate_script(
          "document.activeElement.closest('[role=alertdialog]') === arguments[0]", dialog
        )
        expect(active_element_in_dialog).to be(true)
      end

      (tabbable_count + 1).times do
        page.active_element.send_keys(%i[shift tab])
        active_element_in_dialog = page.evaluate_script(
          "document.activeElement.closest('[role=alertdialog]') === arguments[0]", dialog
        )
        expect(active_element_in_dialog).to be(true)
      end
    end

    it 'closes the account and prevents future login' do
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_on 'Close Account'
      expect(page).to have_text('Are you absolutely sure?', wait: 10)

      fill_in 'Password', with: 'password'
      click_on 'Yes, delete my account'

      expect(page).to have_current_path('/login')
      expect(account.reload).to be_closed
      expect(person.reload.account).to be_nil

      fill_in 'Email address', with: account.email
      fill_in 'Password', with: 'password'
      click_on 'Sign In'

      expect(page).to have_no_current_path('/dashboard')
      expect(page).to have_text(/closed|invalid|error/i)
    end
  end
end
