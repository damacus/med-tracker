# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication pause workflow', :browser do
  fixtures :accounts, :people, :users, :locations, :medications, :carer_relationships, :schedules

  let(:person) { people(:child_user_person) }
  let(:schedule) { schedules(:child_schedule) }
  let(:person_medication) do
    PersonMedication.create!(
      person: person,
      medication: medications(:vitamin_d),
      dose_amount: 10,
      dose_unit: 'mg',
      administration_kind: 'routine'
    )
  end

  before do
    driven_by(:playwright)
    login_as(users(:parent))
  end

  {
    'schedule' => lambda do |example|
      [example.schedule,
       example.pause_form_person_schedule_path(example.person.household.slug, example.person, example.schedule)]
    end,
    'person medication' => lambda do |example|
      [example.person_medication,
       example.pause_form_person_person_medication_path(
         example.person.household.slug, example.person, example.person_medication
       )]
    end
  }.each do |source_name, source_and_path|
    it "pauses a #{source_name} from the directly opened form" do
      source, form_path = source_and_path.call(self)

      visit form_path
      select 'Out of supply', from: 'Reason'
      within('form[action$="/pause"]') { click_button 'Pause' }

      expect(page).to have_current_path(person_path(person))
      expect(source.reload).to be_paused
    end

    it "shows validation errors for a #{source_name} on the directly opened form" do
      source, form_path = source_and_path.call(self)

      visit form_path
      fill_in 'Note (optional)', with: 'Keep this note'
      page.execute_script("document.getElementById('pause-reason').removeAttribute('required')")
      within('form[action$="/pause"]') { click_button 'Pause' }

      expect(page).to have_css('[role="alert"]', text: 'Choose a reason')
      expect(page).to have_field('Note (optional)', with: 'Keep this note')
      expect(source.reload).not_to be_paused
    end
  end

  it 'validates, dismisses, and completes the modal workflow with focus contained' do
    visit person_path(person)

    trigger = find("[data-testid='schedule-actions-#{schedule.id}']")
    trigger.click
    find("[data-testid='pause-schedule-#{schedule.id}']").click

    dialog = find('dialog[open][role="dialog"]', text: 'Pause medication')
    expect(page.evaluate_script("document.activeElement.closest('[role=dialog]') === arguments[0]", dialog)).to be(true)

    click_button 'Cancel'
    expect(page).to have_no_css('[role="dialog"]')

    trigger.click
    find("[data-testid='pause-schedule-#{schedule.id}']").click

    fill_in 'Note (optional)', with: 'Awaiting delivery'
    page.execute_script("document.getElementById('pause-reason').removeAttribute('required')")
    within('dialog[open][role="dialog"]') { click_button 'Pause' }

    expect(page).to have_css('dialog[open][role="dialog"] [role="alert"]', text: 'Choose a reason')
    select 'Out of supply', from: 'Reason'
    within('dialog[open][role="dialog"]') { click_button 'Pause' }

    expect(page).to have_no_css('[role="dialog"]')
    expect(page).to have_text('Schedule paused.')
    expect(schedule.reload).to be_paused
    expect(page.text.scan('Out of supply')).to contain_exactly('Out of supply')
  end
end
