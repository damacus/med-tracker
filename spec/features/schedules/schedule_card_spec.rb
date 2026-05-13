# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedule Card', type: :system do
  fixtures :all

  let(:admin_user) { users(:admin) }
  let(:person) { people(:bob) }
  let(:schedule) { schedules(:bob_aspirin) }
  let(:medication) { schedule.medication }

  before do
    login_as(admin_user)
  end

  it 'opens the edit modal from the schedule card' do
    visit person_path(person)

    within("#schedule_#{schedule.id}") do
      find("[data-testid='edit-schedule-#{schedule.id}']").click
    end

    expect(page).to have_text(/edit schedule/i)
    expect(page).to have_css('div[data-state="open"]')
  end

  it 'updates the persisted dose when changing the selected dose in the edit modal' do
    visit person_path(person)

    within("#schedule_#{schedule.id}") do
      find("[data-testid='edit-schedule-#{schedule.id}']").click
    end

    expect(page).to have_text(/edit schedule/i)
    find('[data-testid="dosage-trigger"]').click
    find('[role="option"]', text: /300(?:\.0)? mg - Standard adult dose/).click
    click_button I18n.t('schedules.form.update_plan')

    expect(page).to have_text(I18n.t('schedules.updated'))
    expect(schedule.reload).to have_attributes(dose_amount: BigDecimal('300'), dose_unit: 'mg')
  end
end
