require "test_helpers/playwright_system_test"

class HomePageTest < PlaywrightSystemTest
  test "visiting the home page" do
    visit root_path

    assert_text "Medicine Tracker Dashboard"
    assert_text "Quick Stats"
    assert_text "Quick Actions"
  end
end
