require 'rails_helper'

RSpec.describe 'Viewing routine dose reports', :browser do
  fixtures :households, :accounts, :people, :users, :locations, :medications, :dosages

  before do
    driven_by(:playwright)
    sign_in(users(:john))
    source = create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                                  dose_cycle: :monthly, max_daily_doses: 2, created_at: 2.months.ago)
    create(:medication_take, person_medication: source, taken_at: Time.current)
  end

  [1280, 390].each do |width|
    it "shows monthly cycle progress at #{width}px" do
      page.current_window.resize_to(width, 844)
      visit reports_path(start_date: Date.current.beginning_of_month, end_date: Date.current)

      within('#routine-cycles') do
        expect(page).to have_text('A cycle is not missed until it ends.')
        expect(page).to have_text(medications(:vitamin_c).display_name)
        expect(page).to have_text(Date.current.end_of_month.iso8601)
        expect(page).to have_css('dd', exact_text: '1')
      end
    end
  end

  it 'shows an insight for repeated unexplained completed routine cycles' do
    create(:person_medication, :routine, person: people(:john), dose_cycle: :weekly,
                                         max_daily_doses: 2, created_at: 2.months.ago)
    page.current_window.resize_to(390, 844)
    visit reports_path(start_date: 6.weeks.ago.to_date, end_date: Date.current)

    expect(page).to have_text('Missed routine cycles')
    expect(page).to have_text('Review the routine dose records for these cycles.')
  end
end
