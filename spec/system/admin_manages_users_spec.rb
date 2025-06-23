require 'rails_helper'

RSpec.describe "AdminManagesUsers", type: :system do
  let!(:admin) { User.create!(name: 'Admin User', date_of_birth: '1980-01-01', email_address: 'admin@example.com', password: 'password', password_confirmation: 'password', role: :admin) }
  let!(:carer) { User.create!(name: 'Carer User', date_of_birth: '1990-01-01', email_address: 'carer@example.com', password: 'password', password_confirmation: 'password', role: :carer) }

  before do
    driven_by(:rack_test)
  end

  context "when logged in as an admin" do
    it "allows admin to see the list of users" do
      # Sign in as admin
      visit login_path
      fill_in "Email address", with: admin.email_address
      fill_in "Password", with: 'password'
      click_button "Sign in"

      # Visit admin users page
      visit admin_users_path

      expect(page).to have_content("User Management")
      expect(page).to have_content(admin.email_address)
      expect(page).to have_content(carer.email_address)
    end
  end

  context "when logged in as a non-admin" do
    it "denies access to the user list" do
      # Sign in as carer
      visit login_path
      fill_in "Email address", with: carer.email_address
      fill_in "Password", with: 'password'
      click_button "Sign in"

      # Visit admin users page
      visit admin_users_path

      expect(page).to have_content("You are not authorized to perform this action.")
      expect(page).to have_current_path(root_path)
    end
  end
end
