# frozen_string_literal: true

require 'test_helpers/playwright_system_test'

class AddAndRemoveMedicineTest < PlaywrightSystemTest
  def setup
    super
    @person = people(:adult_john)
  end

  test 'can add Calpol prescription' do
    visit person_path(@person)

    # Click add prescription button
    click_on 'Add Prescription'

    # Select Calpol from medicine dropdown
    select 'Calpol', from: 'Medicine'

    # Select the 2.5 ml dosage
    select '2.5 ml - Standard child dose', from: 'Dosage'

    # Fill in other required fields
    fill_in 'Frequency', with: 'Every 4-6 hours'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: (Date.current + 7.days).strftime('%Y-%m-%d')

    # Submit the form
    click_on 'Add Prescription'

    # Verify the prescription was created with correct dosage
    assert_text 'Prescription was successfully created'
    assert_text 'Calpol'
    assert_text '2.5 ml'
    assert_text 'Every 4-6 hours'
  end

  test 'can add Ibuprofen prescription' do
    visit person_path(@person)

    # Click add prescription button
    click_on 'Add Prescription'

    # Select Ibuprofen from medicine dropdown
    select 'Ibuprofen', from: 'Medicine'

    # Select the 400 mg dosage
    select '400 mg - Standard adult dose', from: 'Dosage'

    # Fill in other required fields
    fill_in 'Frequency', with: 'Every 6-8 hours'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: (Date.current + 7.days).strftime('%Y-%m-%d')

    # Submit the form
    click_on 'Add Prescription'

    # Verify the prescription was created with correct dosage
    assert_text 'Prescription was successfully created'
    assert_text 'Ibuprofen'
    assert_text '400 mg'
    assert_text 'Every 6-8 hours'
  end

  test 'can remove a prescription' do
    # Create a prescription first
    visit person_path(@person)
    click_on 'Add Prescription'
    select 'Calpol', from: 'Medicine'
    fill_in 'Frequency', with: 'Every 4-6 hours'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: (Date.current + 7.days).strftime('%Y-%m-%d')
    click_on 'Add Prescription'

    # Verify it was created
    assert_text 'Calpol'

    # Find and click the delete button
    within('.prescription', text: 'Calpol') do
      accept_confirm do
        click_on 'Delete'
      end
    end

    # Verify the prescription was deleted
    assert_text 'Prescription was successfully deleted'
    refute_text 'Calpol'
  end

  test 'user does not need to specify amount when taking medicine with default dosage' do
    # Create a prescription with default dosage
    visit person_path(@person)
    click_on 'Add Prescription'
    select 'Calpol', from: 'Medicine'
    fill_in 'Frequency', with: 'Every 4-6 hours'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: (Date.current + 7.days).strftime('%Y-%m-%d')
    click_on 'Add Prescription'

    # The take medicine form should have the default amount pre-filled
    within('.prescription', text: 'Calpol') do
      # Hover to reveal the take medicine form
      find('.prescription__take-trigger').hover

      # Verify the amount field is pre-filled with default dosage
      amount_field = find_field('Amount (ml)')
      assert_equal '2.5', amount_field.value

      # User can just click Take Now without changing the amount
      click_on 'Take Now'
    end

    # Verify the medicine was taken with the default amount
    assert_text 'Medicine taken successfully'
    within('.prescription', text: 'Calpol') do
      assert_text '2.5 ml'
    end
  end
end
