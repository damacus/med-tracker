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
    visit person_path(person)
  end

  it 'allows taking a dose from the card and updates the card state' do
    schedule.medication_takes.destroy_all
    visit person_path(person)

    within("#schedule_#{schedule.id}") do
      expect(page).to have_content(I18n.t('schedules.card.no_doses_today'))
      click_button I18n.t('schedules.card.give')
      expect(page).to have_no_content(I18n.t('schedules.card.no_doses_today'))
    end

    expect(schedule.reload.medication_takes.count).to eq(1)
  end

  it 'opens the edit modal from the schedule card' do
    within("#schedule_#{schedule.id}") do
      find("a[href='#{edit_person_schedule_path(person, schedule)}']").click
    end

    expect(page).to have_content(/edit schedule/i)
    expect(page).to have_css('div[data-state="open"]')
  end

  it 'updates the persisted dose when changing the selected dose in the edit modal' do
    within("#schedule_#{schedule.id}") do
      find("a[href='#{edit_person_schedule_path(person, schedule)}']").click
    end

    expect(page).to have_content(/edit schedule/i)
    find('[data-testid="dosage-trigger"]').click
    find('[role="option"]', text: /300(?:\.0)? mg - Standard adult dose/).click
    click_button I18n.t('schedules.form.update_plan')

    expect(page).to have_content(I18n.t('schedules.updated'))
    expect(schedule.reload).to have_attributes(dose_amount: BigDecimal('300'), dose_unit: 'mg')
  end

  it 'renders a disabled blocked-state take button when the medication is out of stock' do
    medication.update!(current_supply: 0)
    visit person_path(person)

    within("#schedule_#{schedule.id}") do
      expect(page).to have_button(I18n.t('schedules.card.out_of_stock'), disabled: true)
    end
  end
end
