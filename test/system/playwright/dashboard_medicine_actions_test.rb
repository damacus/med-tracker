# frozen_string_literal: true

require 'test_helpers/playwright_system_test'

class DashboardMedicineActionsTest < PlaywrightSystemTest
  fixtures :users, :people, :medicines, :dosages, :prescriptions

  setup do
    @admin = users(:admin)
    perform_sign_in(@admin)
  end

  test 'taking medicine from dashboard' do
    visit dashboard_path

    # Verify dashboard loaded
    assert_text 'Dashboard'
    assert_text 'Medication Schedule'

    # Find a prescription row and take medicine
    prescription = prescriptions(:active_prescription)
    within "#prescription_#{prescription.id}" do
      assert_text prescription.medicine.name
      click_button 'Take Now'
    end

    # Verify we're redirected and medicine was taken
    assert_text 'Medication taken successfully', wait: 5
  end

  test 'taking medicine multiple times' do
    prescription = prescriptions(:active_prescription)
    initial_count = MedicationTake.where(prescription: prescription).count

    # Take medicine three times
    3.times do
      visit dashboard_path

      within "#prescription_#{prescription.id}" do
        click_button 'Take Now'
      end

      assert_text 'Medication taken successfully', wait: 5
    end

    # Verify all takes were recorded
    final_count = MedicationTake.where(prescription: prescription).count
    assert_equal initial_count + 3, final_count
  end

  test 'admin can see delete button' do
    visit dashboard_path

    # Verify delete buttons are visible for admin
    assert_button 'Delete'
  end

  test 'deleting prescription removes it from dashboard' do
    visit dashboard_path

    prescription = prescriptions(:active_prescription)
    prescription_id = prescription.id

    # Verify prescription exists
    assert_selector "#prescription_#{prescription_id}"

    # Click the delete button to open dialog
    within "#prescription_#{prescription_id}" do
      click_button 'Delete'
    end

    # Wait for dialog to appear and become visible
    assert_text 'Delete Prescription?', wait: 5
    assert_text 'Are you sure you want to delete', wait: 2

    # Click the confirm delete button in the dialog
    within '[role="alertdialog"]' do
      click_button 'Delete'
    end

    # Wait for deletion to complete
    sleep 1

    # Verify prescription is removed
    assert_no_selector "#prescription_#{prescription_id}"
  end

  private

  def perform_sign_in(user)
    visit login_path
    fill_in 'email_address', with: user.email_address
    fill_in 'password', with: 'password'
    click_button 'Sign in'
    # Root path now points to dashboard
    assert_current_path root_path
  end
end
