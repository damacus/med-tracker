require "test_helpers/playwright_system_test"

class HomePageTest < PlaywrightSystemTest
  test "visiting the home page" do
    visit root_path

    assert_text "Medicine Tracker"
    assert_text "Medicines"
    assert_text "People"
  end
end
