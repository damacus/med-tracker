# frozen_string_literal: true

require 'test_helpers/playwright_system_test'

class DashboardInteractionsTest < PlaywrightSystemTest
  fixtures :users, :people, :medicines, :dosages, :prescriptions

  setup do
    @admin = users(:admin)
    perform_sign_in(@admin)
  end

  test 'taking medicine multiple times from dashboard' do
    visit dashboard_path

    # Verify dashboard loaded
    assert_text 'Dashboard'
    assert_text 'Medication Schedule'

    # Find a prescription row
    prescription = prescriptions(:active_prescription)
    within "#prescription_#{prescription.id}" do
      assert_text prescription.medicine.name
      assert_text prescription.person.name

      # Take medicine first time
      click_link 'Take Now'
    end

    # Verify we're redirected and medicine was taken
    assert_text 'Medicine taken successfully', wait: 5

    # Go back to dashboard
    visit dashboard_path

    # Take the same medicine again
    within "#prescription_#{prescription.id}" do
      click_link 'Take Now'
    end

    # Verify second take was recorded
    assert_text 'Medicine taken successfully', wait: 5

    # Go back to dashboard for third take
    visit dashboard_path

    # Take medicine third time
    within "#prescription_#{prescription.id}" do
      click_link 'Take Now'
    end

    # Verify third take was recorded
    assert_text 'Medicine taken successfully', wait: 5

    # Verify multiple takes were recorded
    assert_equal 3, MedicationTake.where(prescription: prescription).count
  end

  test 'deleting prescription from dashboard' do
    visit dashboard_path

    # Verify dashboard loaded
    assert_text 'Dashboard'
    assert_text 'Medication Schedule'

    # Find a prescription to delete
    prescription = prescriptions(:active_prescription)
    prescription_id = prescription.id

    within "#prescription_#{prescription_id}" do
      assert_text prescription.medicine.name

      # Click delete and accept confirmation
      page.accept_confirm do
        click_link 'Delete'
      end
    end

    # Wait for deletion to complete
    sleep 1

    # Verify prescription is removed from the page
    assert_no_selector "#prescription_#{prescription_id}"

    # Verify prescription was deleted from database
    assert_nil Prescription.find_by(id: prescription_id)
  end

  test 'taking medicine and then deleting prescription' do
    visit dashboard_path

    prescription = prescriptions(:active_prescription)
    prescription_id = prescription.id

    # First, take the medicine
    within "#prescription_#{prescription_id}" do
      click_link 'Take Now'
    end

    assert_text 'Medicine taken successfully', wait: 5

    # Verify take was recorded
    assert_equal 1, MedicationTake.where(prescription: prescription).count

    # Go back to dashboard
    visit dashboard_path

    # Now delete the prescription
    within "#prescription_#{prescription_id}" do
      page.accept_confirm do
        click_link 'Delete'
      end
    end

    # Wait for deletion
    sleep 1

    # Verify prescription is gone
    assert_no_selector "#prescription_#{prescription_id}"
    assert_nil Prescription.find_by(id: prescription_id)

    # Verify medication takes are also deleted (cascade)
    assert_equal 0, MedicationTake.where(prescription_id: prescription_id).count
  end

  test 'non-admin users cannot delete prescriptions' do
    # Sign out admin and sign in as carer
    click_button 'Sign out'
    perform_sign_in users(:carer)

    visit dashboard_path

    # Verify dashboard loaded
    assert_text 'Dashboard'

    # Verify no delete buttons are visible
    assert_no_link 'Delete'

    # But Take Now buttons should be visible
    assert_link 'Take Now'
  end

  test 'dashboard shows updated prescription count after deletion' do
    visit dashboard_path

    # Get initial count from the stats card
    initial_count_text = first('p', text: /^\d+$/).text
    initial_count = initial_count_text.to_i

    # Delete a prescription
    prescription = prescriptions(:active_prescription)
    within "#prescription_#{prescription.id}" do
      page.accept_confirm do
        click_link 'Delete'
      end
    end

    # Wait for deletion
    sleep 1

    # Refresh to see updated stats
    visit dashboard_path

    # Verify count decreased
    new_count_text = first('p', text: /^\d+$/).text
    new_count = new_count_text.to_i

    assert_equal initial_count - 1, new_count
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
