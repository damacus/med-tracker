require 'rails_helper'

RSpec.describe "UserSignsIns", type: :system do
    let!(:user) { User.create(name: 'Test User', date_of_birth: '2000-01-01', email_address: 'test@example.com', password: 'password', password_confirmation: 'password') }

  before do
    driven_by(:rack_test)
  end

  it "allows a user to sign in" do
    visit login_path

    fill_in "Email address", with: user.email_address
    fill_in "Password", with: 'password'

    click_button "Sign in"

    expect(page).to have_content("Signed in successfully.")
    expect(page).to have_current_path(root_path)
  end
end
