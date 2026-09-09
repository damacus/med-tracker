require 'rails_helper'

RSpec.describe 'Recording routine doses as not taken', :browser do
  fixtures :households, :accounts, :people, :users, :locations, :medications, :dosages

  let(:source) do
    create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                         dose_cycle: :monthly, max_daily_doses: 1, created_at: 2.months.ago)
  end

  before do
    driven_by(:playwright)
    sign_in(users(:john))
    source
  end

  [1280, 390].each do |width|
    it "confirms a routine decision at #{width}px and returns to the dashboard" do
      page.current_window.resize_to(width, 844)
      visit dashboard_path
      click_link 'Not taken', href: new_person_medication_dose_occurrence_path(source), exact: true
      expect(page).to have_css('h1', text: 'Record not taken')
      window = "#{Date.current.beginning_of_month.iso8601} to #{Date.current.end_of_month.iso8601}"
      expect(page).to have_select('Dose', with_options: ["Dose 1 · #{window}"])
      select 'Unwell', from: 'Reason (optional)'
      fill_in 'Note (optional)', with: 'Routine dose declined'
      click_button 'Confirm not taken'
      expect(page).to have_css('[data-testid="dashboard-not-taken-outcome"]', text: 'Routine dose declined')
      expect(page).to have_no_css('[data-testid="dashboard-routine-task"]', text: source.medication.display_name)
    end
  end
end
