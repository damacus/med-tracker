# frozen_string_literal: true

require 'test_helpers/playwright_system_test'

class PrescriptionsTest < PlaywrightSystemTest
  def setup
    super
    @person = people(:adult_john)
    @medicine = medicines(:paracetamol)
  end

  test 'can view a prescription with dosage information' do
    prescription = prescriptions(:john_paracetamol)
    visit person_path(prescription.person)

    # Verify prescription details are displayed
    assert_text prescription.medicine.name
    assert_text "Dosage: #{prescription.dosage.amount} #{prescription.dosage.unit}"
    assert_text prescription.frequency

    # Verify the take medicine form is in a hover card (initially hidden)
    within "#prescription_#{prescription.id}" do
      # The form should be hidden initially
      assert_not find('.prescription__take-form', visible: :all).visible?

      # Hover over the trigger to show the form
      find('.prescription__take-trigger').hover

      # Now the form should be visible
      assert find('.prescription__take-form').visible?

      # Verify the form has the correct default amount
      amount_field = find_field('Amount (ml)')
      assert_equal prescription.dosage.amount.to_s, amount_field.value
    end
  end

  test 'can create a new prescription' do
    visit person_path(@person)

    # Click add prescription button
    click_on 'Add Prescription'

    # Fill in the form
    select @medicine.name, from: 'Medicine'
    fill_in 'Dosage', with: '1.0'
    fill_in 'Frequency', with: 'Every 6 hours'
    fill_in 'Start date', with: Date.current.strftime('%Y-%m-%d')
    fill_in 'End date', with: (Date.current + 7.days).strftime('%Y-%m-%d')
    fill_in 'Notes', with: 'Test prescription notes'

    # Submit the form
    click_on 'Add Prescription'

    # Verify the prescription was created
    assert_text 'Prescription was successfully created'
    assert_text @medicine.name
    assert_text 'Every 6 hours'
  end

  # test "shows validation errors for invalid prescription" do
  #   visit person_path(@person)

  #   # Click add prescription button
  #   click_on "Add Prescription"

  #   # Submit empty form
  #   click_on "Add Prescription"

  #   # Verify validation errors
  #   assert_text "prohibited this prescription from being saved"
  #   assert_text "Medicine must be selected"
  #   assert_text "Dosage can't be blank"
  #   assert_text "Frequency can't be blank"
  #   assert_text "Start date can't be blank"
  # end

  # test "can update an existing prescription" do
  #   prescription = prescriptions(:john_paracetamol)
  #   visit person_path(@person)

  #   # Find and click edit button for the prescription
  #   within("#prescription_#{prescription.id}") do
  #     click_on "Edit"
  #   end

  #   # Update the form
  #   fill_in "Dosage", with: "2.0"
  #   fill_in "Notes", with: "Updated prescription notes"

  #   # Submit the form
  #   click_on "Update Prescription"

  #   # Verify the changes
  #   assert_text "Prescription was successfully updated"
  #   assert_text "2.0"
  #   assert_text "Updated prescription notes"
  # end

  # test "can delete a prescription" do
  #   prescription = prescriptions(:john_paracetamol)
  #   visit person_path(@person)

  #   # Find and click delete button for the prescription
  #   within("#prescription_#{prescription.id}") do
  #     accept_confirm do
  #       click_on "Delete"
  #     end
  #   end

  #   # Verify the prescription was deleted
  #   assert_text "Prescription was successfully deleted"
  #   refute_text prescription.medicine.name
  # end
end
