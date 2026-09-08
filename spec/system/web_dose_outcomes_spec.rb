require 'rails_helper'

RSpec.describe 'Recording a dose as not taken', :browser do
  fixtures :households, :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  before do
    driven_by(:playwright)
    sign_in(users(:john))
    schedules(:john_movicol).update!(frequency: 'Daily', schedule_type: :daily, schedule_config: {},
                                     start_date: Date.yesterday, end_date: Date.tomorrow, max_daily_doses: 1)
  end

  [1280, 390].each do |width|
    it "confirms a not-taken dose at #{width}px and shows the decision on the dashboard" do
      page.current_window.resize_to(width, 844)
      visit dashboard_path
      click_link 'Not taken', href: new_schedule_dose_occurrence_path(schedules(:john_movicol)), exact: true
      expect(page).to have_css('h1', text: 'Record not taken')
      select 'Unwell', from: 'Reason (optional)'
      fill_in 'Note (optional)', with: 'Feeling unwell'
      click_button 'Confirm not taken'
      expect(page).to have_css('[data-testid="dashboard-not-taken-outcome"]', text: 'Feeling unwell')
      expect(page).to have_no_css('[data-testid="dashboard-routine-task"]',
                                  text: schedules(:john_movicol).medication.display_name)
    end
  end
end
