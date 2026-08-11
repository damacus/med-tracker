# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Schedules workflow' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:person) { people(:bob) }
  let(:schedule) { schedules(:bob_aspirin) }
  let(:selected_dosage_option) do
    MedicationDosageOption.find_by!(
      medication: schedule.medication,
      amount: schedule.dose_amount,
      unit: schedule.dose_unit
    )
  end

  before do |example|
    driven_by(example.metadata[:js] ? :playwright : :rack_test)
    sign_in(users(:admin))
  end

  it 'is discoverable from medication details' do
    visit medication_path(medications(:paracetamol))

    click_on 'Schedule'

    expect(page).to have_current_path(add_medication_path(medication_id: medications(:paracetamol).id))
  end

  it 'routes workflow selections into a prefilled schedule form' do
    visit schedules_workflow_path

    select 'Prescribed', from: 'Type (OTC or prescribed)'
    select 'John Doe', from: 'Person name'
    select 'Paracetamol', from: 'Name of med'
    fill_in 'Dose, frequency', with: 'Twice daily'

    click_button 'Continue to schedule details'

    expect(page).to have_current_path(
      new_person_schedule_path(
        people(:john),
        medication_id: medications(:paracetamol).id,
        frequency: 'Twice daily',
        schedule_type: 'prescribed'
      )
    )
    expect(page).to have_text('Add schedule for John Doe')
  end

  it 'keeps native invalid feedback for a required field without an error target', :js do
    visit schedules_workflow_path

    click_button 'Continue to schedule details'
    invalid_event_completed = page.evaluate_script(<<~JS)
      document.querySelector('#person_id').dispatchEvent(
        new Event('invalid', { cancelable: true })
      )
    JS

    aggregate_failures do
      expect(invalid_event_completed).to be(true)
      expect(page.evaluate_script('document.querySelector("#person_id").matches(":invalid")')).to be(true)
      expect(page).to have_no_css('#person_id[aria-invalid]')
    end
  end

  it 'shows native required feedback when an end date is cleared', :js do
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

  it 'updates an unchanged schedule with its selected dosage', :js do
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
      open_schedule_actions
      edit_schedule
    end

    expect(page).to have_text(/edit schedule/i)
  end

  def open_schedule_actions
    find("[data-testid='schedule-actions-#{schedule.id}']").click
  end

  def edit_schedule
    find("[data-testid='edit-schedule-#{schedule.id}']").click
  end
end
