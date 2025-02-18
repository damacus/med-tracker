require "test_helpers/playwright_system_test"

class DashboardTest < PlaywrightSystemTest
  test "visiting the dashboard" do
    visit dashboard_path

    # Test page title
    assert_text "Medicine Tracker Dashboard"

    # Test quick stats
    assert_text "Quick Stats"
    assert_text "People"
    assert_text "3" # Three people
    assert_text "Active Prescriptions"
    assert_text "3" # Three active prescriptions

    # Test medication schedule
    assert_text "Medication Schedule By Person"
  end
end
