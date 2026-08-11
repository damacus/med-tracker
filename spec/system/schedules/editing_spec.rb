# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedule editing', :browser do
  fixtures :all

  let(:admin_user) { users(:admin) }
  let(:person) { people(:bob) }
  let(:schedule) { schedules(:bob_aspirin) }
  let(:selected_dosage_option) do
    MedicationDosageOption.find_by!(medication: schedule.medication, amount: schedule.dose_amount, unit: schedule.dose_unit)
  end

  before do
    login_as(admin_user)
  end

  it 'shows native required feedback when an end date is cleared' do
    open_edit_form

    expect(page).to have_css('label[for="schedule_end_date"]', text: /end date \*/i)
    expect(page).to have_css('#schedule_end_date[required]')

    fill_in 'End date', with: ''
    click_button I18n.t('schedules.form.update_plan')

    aggregate_failures do
      expect(page).to have_css('#schedule_end_date[aria-invalid="true"][aria-describedby="schedule_end_date_error"]')
      expect(page).to have_css('#schedule_end_date_error[role="alert"]', text: /\S/)
      expect(page.evaluate_script('document.querySelector("#schedule_end_date").matches(":invalid")')).to be(true)
    end
  end

  it 'updates an unchanged schedule with its selected dosage' do
    open_edit_form

    expect(page).to have_field(
      'schedule_dose_option_key',
      with: selected_dosage_option.id.to_s,
      visible: :all
    )

    click_button I18n.t('schedules.form.update_plan')

    aggregate_failures do
      expect(page).to have_text(I18n.t('schedules.updated'))
      expect(schedule.reload).to have_attributes(
        dose_amount: BigDecimal('75'),
        dose_unit: 'mg',
        source_dosage_option_id: selected_dosage_option.id
      )
    end
  end

  private

  def open_edit_form
    visit person_path(person)

    within("##{tenant_dom_id(schedule)}") do
      find("[data-testid='schedule-actions-#{schedule.id}']").click
      find("[data-testid='edit-schedule-#{schedule.id}']").click
    end

    expect(page).to have_text(/edit schedule/i)
  end
end
