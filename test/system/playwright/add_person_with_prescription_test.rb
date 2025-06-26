# frozen_string_literal: true

require 'system/playwright/test_helper'

class AddPersonWithPrescriptionTest < ApplicationSystemTestCase
  test 'can add a new person and prescribe medication' do
    visit people_path

    # Add a new person
    click_on 'Add Person'
    assert_selector '#modal .modal'
    within '#modal .modal' do
      fill_in 'Name', with: 'Dale'
      fill_in 'Date of birth', with: '1990-03-10'
      click_on 'Create Person'
    end

    # Verify person was created
    assert_text 'Person was successfully created'
    assert_text 'Dale'
    assert_text 'Age: 34'

    # Add a prescription
    within '#people' do
      within 'div.person-card', text: 'Dale' do
        click_on 'Add Prescription'
      end
    end
    sleep 0.5 # Wait for Turbo Stream to update
    assert_selector '#modal .modal'
    within '#modal .modal' do
      select 'Calpol', from: 'Medicine'
      fill_in 'Dosage', with: '5'
      fill_in 'Frequency', with: 'Every 4 hours'
      fill_in 'Start date', with: Time.current.strftime('%Y-%m-%d')
      click_on 'Add Prescription'
    end

    # Verify prescription was added
    assert_text 'Calpol'
    assert_text '5'
    assert_text 'Every 4 hours'
  end
end
