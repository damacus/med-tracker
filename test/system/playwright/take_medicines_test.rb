# frozen_string_literal: true

require 'system/playwright/test_helper'

class TakeMedicinesTest < ApplicationSystemTestCase
  test 'recording a medication take' do
    fixed_time = Time.zone.local(2025, 2, 4, 10, 0, 0)
    travel_to fixed_time do
      # Create test data
      person = Person.create!(
        name: 'John Doe',
        date_of_birth: '1990-01-01'
      )

      medicine = Medicine.create!(
        name: 'Test Medicine',
        description: 'Test Description',
        dosage: '5.0',
        unit: 'ml'
      )

      Prescription.create!(
        person: person,
        medicine: medicine,
        dosage: '5.0ml',
        frequency: 'daily',
        start_date: Date.current
      )

      visit person_path(person)

      # Initial state - no medications taken today
      assert_text 'No medications taken today'

      # Take medication
      fill_in 'amount_ml', with: '5.0'
      click_button 'Take Now'

      # After navigation, verify the take was recorded
      assert_text 'Medicine taken successfully', wait: 5
      assert_text "Today's takes:", wait: 5
      assert_text '10:00 AM', wait: 5
      assert_no_text 'No medications taken today'
    end
  end
end
