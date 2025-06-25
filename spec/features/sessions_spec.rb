# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Sessions", type: :feature do
  describe "login page" do
    before do
      visit login_path
    end

    it "displays the login form" do
      expect(page).to have_field("Email address")
      expect(page).to have_field("Password")
      expect(page).to have_button("Sign in")
      expect(page).to have_link("Forgot password?")
    end

    it "maintains the email value after a failed attempt" do
      fill_in "Email address", with: "test@example.com"
      fill_in "Password", with: "wrongpassword"
      click_button "Sign in"

      expect(page).to have_field("Email address", with: "test@example.com")
    end

    # Note: Flash messages will be handled by ruby-ui, not tested here
  end
end
