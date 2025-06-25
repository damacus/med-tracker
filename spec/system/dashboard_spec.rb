require "rails_helper"

RSpec.describe "Dashboard", type: :system do
  fixtures :users, :medicines, :dosages, :prescriptions

  it "loads the dashboard for a signed-in user" do
    # Sign in the user from fixtures
    sign_in(users(:john))

    # Visit the dashboard path
    visit dashboard_path

    # Assert that the page content is correct
    expect(page).to have_content("Medicine Tracker Dashboard")
    expect(page).to have_link("Add Medicine", href: new_medicine_path)
    # expect(page).to have_link("Add Person", href: new_user_path)
  end
end
