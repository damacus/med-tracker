# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Demo mode notice', :browser do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications,
           :dosages, :schedules, :person_medications, :medication_takes

  let(:profile_notice_geometry_script) do
    <<~JS
      (() => {
        const demo = document.querySelector('[role="status"][aria-labelledby="demo-environment-title"]');
        const reminder = Array.from(document.querySelectorAll('[role="alert"]')).find((element) =>
          element.textContent.includes('For enhanced security, please set up two-factor authentication')
        );
        if (!demo || !reminder) {
          return { missing: [!demo && 'demo', !reminder && 'reminder'].filter(Boolean) };
        }
        const demoRect = demo.getBoundingClientRect();
        const reminderRect = reminder.getBoundingClientRect();
        const overlap = demoRect.left < reminderRect.right && reminderRect.left < demoRect.right &&
          demoRect.top < reminderRect.bottom && reminderRect.top < demoRect.bottom;
        const visible = [demoRect, reminderRect].every((rect) =>
          rect.top >= 0 && rect.left >= 0 && rect.right <= window.innerWidth && rect.bottom <= window.innerHeight
        );
        const content = [demo.querySelector('#demo-environment-title'), reminder];
        const unobscured = content.every((element) => {
          const rect = element.getBoundingClientRect();
          return element.contains(document.elementFromPoint(rect.left + (rect.width / 2), rect.top + (rect.height / 2)));
        });

        return { missing: [], overlap, visible: visible && unobscured };
      })()
    JS
  end

  let(:profile_flash_viewport_script) do
    <<~JS
      (() => {
        const stack = document.querySelector('[data-testid="notice-stack"]');
        if (!stack) return { missing: true };
        const rect = stack.getBoundingClientRect();

        return {
          missing: false,
          fixed: getComputedStyle(stack).position === 'fixed',
          visible: rect.top >= 0 && rect.left >= 0 && rect.right <= window.innerWidth && rect.bottom <= window.innerHeight
        };
      })()
    JS
  end

  around do |example|
    original_value = ENV.fetch('DEMO_MODE', nil)
    example.run
  ensure
    original_value.nil? ? ENV.delete('DEMO_MODE') : ENV['DEMO_MODE'] = original_value
  end

  it 'does not identify ordinary deployments as demo environments' do
    ENV.delete('DEMO_MODE')
    sign_in(users(:admin))

    visit dashboard_path

    expect(page).to have_current_path(dashboard_path)
    expect(page).to have_no_text('Demo environment')
  end

  it 'keeps the demo identity visible without changing medication access' do
    ENV['DEMO_MODE'] = 'true'

    travel_to(Time.current.beginning_of_day + 9.hours) do
      sign_in(users(:jane))
      visit dashboard_path

      expect(page).to have_css('[role="status"]', text: 'Demo environment')
      expect(page).to have_text('Every Sunday at 04:15 Europe/London')
      expect(page).to have_text("Today's Schedule")
      expect(page).to have_css('[data-testid^="take-dose-"]')
    end
  end

  it 'keeps the demo and two-factor notices visible without overlap at desktop and mobile widths' do
    ENV['DEMO_MODE'] = 'true'
    user = users(:damacus)
    clear_2fa_for_account(user.person.account)
    login_as(user)

    [[1280, 900], [390, 844]].each do |width, height|
      page.current_window.resize_to(width, height)
      visit profile_path

      expect(page).to have_css('[role="status"][aria-labelledby="demo-environment-title"]', text: 'Demo environment')
      expect(page).to have_css('[role="alert"]', text: 'For enhanced security, please set up two-factor authentication')

      expect(profile_notice_geometry).to include('missing' => [], 'overlap' => false, 'visible' => true)
    end
  end

  it 'lets the user dismiss both persistent notices' do
    ENV['DEMO_MODE'] = 'true'
    user = users(:damacus)
    clear_2fa_for_account(user.person.account)
    login_as(user)
    visit profile_path

    demo_notice = find('[role="status"][aria-labelledby="demo-environment-title"]')
    reminder = find('[role="alert"]', text: 'For enhanced security, please set up two-factor authentication')

    [demo_notice, reminder].each do |notice|
      geometry = notice.evaluate_script(<<~JS)
        (() => {
          const button = this.querySelector('button[aria-label="Close"]');
          const icon = button?.querySelector('svg');
          const noticeRect = this.getBoundingClientRect();
          const buttonRect = button?.getBoundingClientRect();
          const iconRect = icon?.getBoundingClientRect();

          return {
            button_inside: buttonRect && buttonRect.top >= noticeRect.top && buttonRect.bottom <= noticeRect.bottom,
            icon_visible: iconRect && iconRect.width >= 16 && iconRect.height >= 16
          };
        })()
      JS

      expect(geometry).to include('button_inside' => true, 'icon_visible' => true)
    end

    demo_notice.find('button[aria-label="Close"]').click
    expect(page).to have_no_css('[role="status"][aria-labelledby="demo-environment-title"]')

    reminder.find('button[aria-label="Close"]').click
    expect(page).to have_no_css(
      '[role="alert"]',
      text: 'For enhanced security, please set up two-factor authentication'
    )
  end

  it 'keeps Turbo profile feedback fixed in view after a scrolled mobile update' do
    ENV['DEMO_MODE'] = 'true'
    user = users(:damacus)
    login_as(user)
    page.current_window.resize_to(390, 844)
    visit profile_path

    find('[data-testid="profile-time-zone-dialog"] button', text: 'Time Zone').click
    time_zone = find_by_id('account_time_zone')
    page.execute_script('arguments[0].scrollIntoView({ block: "center" })', time_zone)
    expect(page.evaluate_script('window.scrollY')).to be_positive

    select 'Pacific Time (US & Canada)', from: 'Time Zone'
    click_button 'Save time zone'

    expect(page).to have_text('Profile updated successfully')
    expect(profile_flash_viewport_geometry).to include('missing' => false, 'fixed' => true, 'visible' => true)
  end

  def profile_notice_geometry
    page.evaluate_script(profile_notice_geometry_script)
  end

  def profile_flash_viewport_geometry
    page.evaluate_script(profile_flash_viewport_script)
  end
end
