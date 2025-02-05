require "test_helpers/playwright_system_test"

class DashboardTest < PlaywrightSystemTest
  test "visiting the dashboard" do
    # Create test data
    person = Person.create!(
      name: "John Doe",
      date_of_birth: "1990-01-01"
    )

    medicine = Medicine.create!(
      name: "Test Medicine",
      description: "Test Description",
      standard_dosage: "1 pill"
    )

    prescription = Prescription.create!(
      person: person,
      medicine: medicine,
      dosage: "1 pill",
      frequency: "daily",
      start_date: Date.current,
      end_date: 1.month.from_now
    )

    visit root_path

    # Test page title
    assert_text "Medicine Tracker Dashboard"

    # Test quick stats
    assert_text "Quick Stats"
    assert_text "Total People"
    assert_text "1" # One person
    assert_text "Active Prescriptions"
    assert_text "1" # One active prescription

    # Test medication schedule
    assert_text "Medication Schedule by Person"
    assert_text "John Doe"
    assert_text "Test Medicine"
    assert_text "Dosage: 1 pill"
    assert_text "Frequency: daily"
  end
end
