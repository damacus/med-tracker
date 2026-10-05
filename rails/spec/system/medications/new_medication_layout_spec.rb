# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedicationNewLayout', :browser do
  fixtures :accounts, :people, :users, :locations, :medications

  before do
    driven_by(:playwright)
  end

  it 'allows creating a medication with the redesigned form' do
    sign_in(users(:john))

    visit new_medication_path

    expect(page).to have_text('Add a New Medication')

    within('[data-testid="medication-wizard-form"]') do
      fill_in 'Name', with: 'Ibuprofen'
      fill_in 'Description', with: 'Pain relief'
      click_button 'Continue'

      aggregate_failures 'dose schedule fields' do
        expect(page).to have_text('Who will take this?')
        expect(page).to have_field('Amount')
        expect(page).to have_select('Unit', with_options: %w[mg sachet])
        expect(page).to have_field('wizard_schedule_type_multiple_daily', type: 'radio', visible: :all)
        expect(page).to have_field('wizard_schedule_type_daily', type: 'radio', visible: :all)
        expect(page).to have_field('wizard_schedule_type_weekly', type: 'radio', visible: :all)
        expect(page).to have_field('wizard_schedule_type_specific_dates', type: 'radio', visible: :all)
        expect(page).to have_field('wizard_schedule_type_prn', type: 'radio', visible: :all)
        expect(page).to have_field('wizard_schedule_type_tapering', type: 'radio', visible: :all)
      end

      select 'John Doe', from: 'Who will take this?'
      fill_in 'Amount', with: 200
      select 'mg', from: 'Unit'
      choose_schedule_type('multiple_daily')
      fill_in 'Doses per day', with: 2
      fill_in 'Hours apart', with: 12
      fill_in 'First dose', with: '08:00'
      fill_in 'Second dose', with: '20:00'
      fill_in 'Start date', with: Time.zone.today.to_s
      fill_in 'End date', with: 1.month.from_now.to_date.to_s
      click_button 'Continue'
      expect(page).to have_text('Who will take this?')
      expect(page).to have_no_field('Starting Supply')
      click_button 'Review medication plan'
      expect(page).to have_text('200 mg, Twice daily')
      expect(page).to have_field('medication_schedule_review_complete', with: 'reviewed', visible: :all)
      fill_in 'Hours apart', with: 12
      expect(page).to have_field('medication_schedule_review_complete', with: '', visible: :all)
      click_button 'Continue'
      expect(page).to have_text('Who will take this?')
      expect(page).to have_no_field('Starting Supply')
      click_button 'Review medication plan'
      expect(page).to have_field('medication_schedule_review_complete', with: 'reviewed', visible: :all)
      click_button 'Continue'

      aggregate_failures 'supply fields' do
        expect(page).to have_field('Starting Supply')
        expect(page).to have_field('Reorder Threshold')
      end
      fill_in 'Starting Supply', with: 40
      fill_in 'Reorder Threshold', with: 10
      click_button 'Continue'
      expect(page).to have_field('Warnings')
      fill_in 'Warnings', with: 'Take with food'

      click_button 'Save Medication'
    end

    aggregate_failures 'persistence' do
      expect(page).to have_text('Ibuprofen created!')
      expect(page).to have_link('Manage dose options')
      expect(page).to have_link('Done')
      expect(page).to have_text('200 mg')
      expect(page).to have_text('Twice daily')
    end
  end

  it 'defaults location to the signed-in user primary location' do
    sign_in(users(:jane))

    visit new_medication_path

    checked_location = find("input[name='medication[location_id]'][checked]", visible: :all)

    expect(checked_location.value).to eq(household_location(locations(:home)).id.to_s)
  end

  it 'keeps the wizard readable on a mobile viewport' do
    sign_in(users(:john))
    page.current_window.resize_to(320, 844)

    visit new_medication_path

    geometry = page.evaluate_script(<<~JS)
      (() => {
        const content = document.querySelector('#wizard-content');
        const labels = Array.from(document.querySelectorAll('[data-indicator-label]'));
        const labelBounds = labels.map((label) => label.getBoundingClientRect());

        return {
          contentWidth: content.getBoundingClientRect().width,
          labelsFit: labelBounds.every((bounds, index) => {
            const nextBounds = labelBounds[index + 1];
            return !nextBounds || bounds.right + 4 <= nextBounds.left;
          })
        };
      })()
    JS

    expect(geometry).to include('contentWidth' => be >= 240, 'labelsFit' => true)
  end

  it 'stores one configured time for each multiple-daily dose' do
    sign_in(users(:john))

    visit new_medication_path

    within('[data-testid="medication-wizard-form"]') do
      fill_in 'Name', with: 'Three Times Daily Medicine'
      click_button 'Continue'

      select 'John Doe', from: 'Who will take this?'
      fill_in 'Amount', with: 5
      select 'ml', from: 'Unit'
      choose_schedule_type('multiple_daily')
      fill_in 'Doses per day', with: 3
      fill_in 'Hours apart', with: 6
      fill_in 'First dose', with: '08:00'
      fill_in 'Second dose', with: '14:00'
      fill_in 'Start date', with: Time.zone.today.to_s
      fill_in 'End date', with: 1.month.from_now.to_date.to_s
      click_button 'Review medication plan'
      expect(page).to have_field('medication_schedule_review_complete', with: 'reviewed', visible: :all)
      schedule_config = JSON.parse(find("input[name='onboarding_schedule[schedule_config]']", visible: :all).value)
      expect(schedule_config).to include('times' => %w[08:00 14:00 20:00])
      click_button 'Continue'

      fill_in 'Starting Supply', with: 40
      fill_in 'Reorder Threshold', with: 10
      click_button 'Continue'
      click_button 'Save Medication'
    end

    expect(page).to have_text('Three Times Daily Medicine created!')
    expect(page).to have_text('5 ml')
    expect(page).to have_text('Three times daily')
  end

  it 'allows adding multiple selected dates without typing a comma-separated list' do
    sign_in(users(:john))

    visit new_medication_path

    within('[data-testid="medication-wizard-form"]') do
      fill_in 'Name', with: 'Selected Dates Medicine'
      click_button 'Continue'

      select 'John Doe', from: 'Who will take this?'
      fill_in 'Amount', with: 1
      select 'tablet', from: 'Unit'
      choose_schedule_type('specific_dates')
      fill_in 'Date to add', with: Time.zone.today.to_s
      click_button 'Add date'
      fill_in 'Date to add', with: 2.days.from_now.to_date.to_s
      click_button 'Add date'
      fill_in 'Start date', with: Time.zone.today.to_s
      fill_in 'End date', with: 1.month.from_now.to_date.to_s
      click_button 'Review medication plan'
      expect(page).to have_field('medication_schedule_review_complete', with: 'reviewed', visible: :all)
      schedule_config = JSON.parse(find("input[name='onboarding_schedule[schedule_config]']", visible: :all).value)
      expect(schedule_config).to include('dates' => [Time.zone.today.to_s, 2.days.from_now.to_date.to_s])
      click_button 'Continue'

      fill_in 'Starting Supply', with: 10
      fill_in 'Reorder Threshold', with: 2
      click_button 'Continue'
      click_button 'Save Medication'
    end

    expect(page).to have_text('Selected Dates Medicine created!')
    expect(page).to have_text('1 tablet')
    expect(page).to have_text('Specific dates')
  end

  it 'omits the end date from as-needed medication plan review' do
    sign_in(users(:john))

    visit new_medication_path

    within('[data-testid="medication-wizard-form"]') do
      fill_in 'Name', with: 'As Needed Pain Relief'
      click_button 'Continue'

      select 'John Doe', from: 'Who will take this?'
      fill_in 'Amount', with: 500
      select 'mg', from: 'Unit'
      choose_schedule_type('prn')

      expect(page).to have_field('Start date')
      expect(page).to have_no_field('End date')

      click_button 'Review medication plan'

      expect(page).to have_text('500 mg, As needed for John Doe')
      expect(page).to have_no_text(1.month.from_now.to_date.to_s)
    end
  end

  def choose_schedule_type(type)
    find("label[data-medication-schedule-wizard-target='scheduleTypeCard'][data-schedule-type='#{type}']").click
  end
end
