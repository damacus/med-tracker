# frozen_string_literal: true

require "rails_helper"

RSpec.describe Layouts::Navigation, type: :view do
  fixtures :users

  context "when user is authenticated" do
    it "renders the navigation with a sign out button" do
      render(described_class.new(current_user: users(:john)))

      expect(rendered).to have_link("Medicines", href: medicines_path, visible: :all)
      expect(rendered).to have_link("People", href: people_path, visible: :all)
      expect(rendered).to have_link("Medicine Finder", href: medicine_finder_path, visible: :all)
      expect(rendered).to have_button("Sign out")
      expect(rendered).not_to have_link("Login", href: login_path)
    end
  end

  context "when user is not authenticated" do
    it "renders the navigation with a login link" do
      render(described_class.new(current_user: nil))

      expect(rendered).to have_link("Medicines", href: medicines_path, visible: :all)
      expect(rendered).to have_link("People", href: people_path, visible: :all)
      expect(rendered).to have_link("Medicine Finder", href: medicine_finder_path, visible: :all)
      expect(rendered).to have_link("Login", href: login_path)
      expect(rendered).not_to have_button("Sign out")
    end
  end
end
