# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Demo mode notice', :browser do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications,
           :dosages, :schedules, :person_medications, :medication_takes

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
end
