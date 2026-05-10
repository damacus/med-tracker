# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Navigation Flow" do
  fixtures(:accounts, :people, :users)

  describe "Direct navigation when not logged in" do
    it "redirects to login when accessing profile" do
      visit("/profile")
      expect(page).to(have_current_path("/login"))
    end

    it "redirects to login when accessing dashboard" do
      visit("/dashboard")
      expect(page).to(have_current_path("/login"))
    end
  end
end
