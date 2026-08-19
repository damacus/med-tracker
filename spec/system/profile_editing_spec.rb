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
      open_profile_section('security')
      click_on 'Change', match: :prefer_exact

      expect(page).to have_css('dialog[open][role="dialog"]')
      expect(page).to have_text('Change Login')
    end

    it 'submits email change request when saving' do
      open_profile_section('security')
      click_on 'Change', match: :prefer_exact

      expect(page).to have_css('dialog[open][role="dialog"]')

      fill_in 'New Login', with: 'newemail@example.com'
      fill_in 'Password', with: 'password'
      click_on 'Change Login'

      expect(page).to have_text('An email has been sent to you with a link to verify your login change')
    end
  end

  describe 'changing password', :js do
    it 'opens modal when clicking change' do
      open_profile_section('security')
      click_link 'Change', href: '/change-password'

      expect(page).to have_css('dialog[open][role="dialog"]')
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
      find('[data-testid="profile-time-zone-dialog"] button', text: 'Time Zone').click
      expect(page).to have_select('Time Zone', selected: stored_time_zone)

      click_button 'Save time zone'

      expect(page).to have_text(I18n.t('profiles.updated'))
      expect(account.reload.time_zone).to eq(stored_time_zone)
    end

    it 'saves a deliberately selected time zone' do
      visit profile_path

      find('[data-testid="profile-time-zone-dialog"] button', text: 'Time Zone').click
      select 'Pacific Time (US & Canada)', from: 'Time Zone'
      click_button 'Save time zone'

      expect(page).to have_text(I18n.t('profiles.updated'))
      expect(account.reload.time_zone).to eq('Pacific Time (US & Canada)')
    end
  end

  describe 'closing account', :js do
    it 'shows confirmation dialog when clicking close account' do
      open_profile_section('advanced')
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
      open_profile_section('advanced')
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
      open_profile_section('advanced')
      expect(page).to have_css('[data-ruby-ui--alert-dialog-target="content"]', visible: :hidden, wait: 5)

      click_on 'Close Account'
      expect(page).to have_text('Are you absolutely sure?', wait: 10)

      click_on 'Cancel'

      # Dialog should close and we're still on profile page
      expect(page).to have_text('My Profile')
    end

    it 'keeps keyboard focus inside the account closure alert dialog' do
      open_profile_section('advanced')
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
      open_profile_section('advanced')
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

  describe 'profile tabs and focused editors', :js do
    it 'opens and closes a mobile time-zone dialog without overflow and restores focus' do
      page.current_window.resize_to(390, 844)
      trigger = find('[data-testid="profile-time-zone-dialog"] button', text: 'Time Zone')
      trigger.click

      dialog = find('dialog[open][role="dialog"]', text: 'Time Zone')
      dimensions = dialog.evaluate_script(<<~JS)
        ({ left: this.getBoundingClientRect().left, right: this.getBoundingClientRect().right,
           viewport: window.innerWidth, pageWidth: document.documentElement.scrollWidth })
      JS
      expect(dimensions.fetch('left')).to be >= 0
      expect(dimensions.fetch('right')).to be <= dimensions.fetch('viewport')
      expect(dimensions.fetch('pageWidth')).to eq(dimensions.fetch('viewport'))

      within(dialog) { click_button I18n.t('ruby_ui.common.close'), match: :first }

      expect(page).to have_no_css('[role="dialog"]', text: 'Time Zone')
      expect(page.evaluate_script('document.activeElement === arguments[0]', trigger)).to be(true)
    end

    it 'switches same-page profile tabs without navigating or browser errors', :aggregate_failures do
      browser_errors = []
      page.driver.with_playwright_page do |playwright_page|
        playwright_page.on('console', ->(message) { browser_errors << message.text if message.type == 'error' })
        playwright_page.on('pageerror', ->(error) { browser_errors << error.message })
      end
      expect(find('[data-appearance-summary]').text).to eq('System')
      tab_tops = all('[data-testid="profile-section-tab"]').map do |tab|
        tab.evaluate_script('this.getBoundingClientRect().top')
      end
      expect(tab_tops.uniq.size).to eq(1)
      initial_path = page.current_url
      open_profile_section('security')

      expect(page.current_url).to eq(initial_path)
      expect(page).to have_css('[data-profile-section="security"][aria-selected="true"]')
      expect(page).to have_css('[data-profile-section="profile"][aria-selected="false"]')
      expect(page).to have_css(
        '[data-testid="profile-section-panel"][aria-labelledby="profile-tab-security"]:not(.hidden)', count: 1
      )
      expect(page).to have_css('[data-testid="profile-section-panel"]', count: 4, visible: :all)
      visible_panel = find('[data-testid="profile-section-panel"]:not(.hidden)')
      expect(visible_panel.evaluate_script('this.getBoundingClientRect().width')).to be > 700
      expect(browser_errors).to be_empty
    end

    it 'shows browser notifications as on when a subscription exists' do
      page.driver.with_playwright_page do |playwright_page|
        playwright_page.add_init_script(script: <<~JS)
          Object.defineProperty(window, "PushManager", { configurable: true, value: function PushManager() {} })
          Object.defineProperty(window, "Notification", {
            configurable: true,
            value: { permission: "granted", requestPermission: async () => "granted" }
          })
          Object.defineProperty(navigator, "serviceWorker", {
            configurable: true,
            value: {
              ready: Promise.resolve({
                pushManager: { getSubscription: async () => ({ endpoint: "https://push.example/subscription" }) }
              })
            }
          })
        JS
      end
      visit profile_path(section: 'notifications')

      group = find('[role="radiogroup"][aria-label="Browser Notifications"]')
      expect(group.find('[role="radio"]', text: 'On')['aria-checked']).to eq('true')
      expect(group.find('[role="radio"]', text: 'Off')['aria-checked']).to eq('false')
    end
  end

  def open_profile_section(section)
    find("[data-profile-section='#{section}']").click
  end
end
