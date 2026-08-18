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
        const demoRect = demo.getBoundingClientRect();
        const reminderRect = reminder.getBoundingClientRect();
        const overlap = demoRect.top < reminderRect.bottom && reminderRect.top < demoRect.bottom;
        const visible = [demoRect, reminderRect].every((rect) =>
          rect.top >= 0 && rect.left >= 0 && rect.right <= window.innerWidth && rect.bottom <= window.innerHeight
        );
        const unobscured = [demo, reminder].every((element) => {
          const rect = element.getBoundingClientRect();
          return element.contains(document.elementFromPoint(rect.left + (rect.width / 2), rect.top + (rect.height / 2)));
        });

        return { overlap, visible: visible && unobscured };
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

      expect(profile_notice_geometry).to include('overlap' => false, 'visible' => true)
    end
  end

  def profile_notice_geometry
    page.evaluate_script(profile_notice_geometry_script)
  end
end
