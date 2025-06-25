# frozen_string_literal: true

require "rails_helper"

# Following the Red-Green-Refactor TDD cycle
# This spec tests the navigation component with the official Phlex testing approach
RSpec.describe Components::Layouts::Navigation, type: :component do
  # Create a test user fixture
  let(:user) { double("User", id: 1, email_address: "test@example.com") }
  
  # Test behavior when user is authenticated
  context "when user is authenticated" do
    it "renders navigation with sign out button" do
      # Set up component according to TDD test case
      component = Components::Layouts::Navigation.new(current_user: user)
      
      # Stub URL helpers to isolate test from Rails routing
      stub_url_helpers(component)
      
      # We need to stub form and link helpers
      allow(component).to receive(:link_to) do |text, path|
        "<a href=\"#{path}\">#{text}</a>"
      end
      
      allow(component).to receive(:form_with) do |**kwargs, &block|
        form_content = block.call if block
        "<form action=\"#{kwargs[:url]}\" method=\"#{kwargs[:method] || 'post'}\">#{form_content}</form>"
      end
      
      # Use the Phlex testing approach
      render(component)
      
      # Test the rendered output for authenticated experience
      expect(rendered).to include("Medicines")
      expect(rendered).to include("People")
      expect(rendered).to include("Medicine Finder")
      expect(rendered).to include("Sign out")
      expect(rendered).not_to include("Login")
    end
  end
  
  # Test behavior when user is not authenticated
  context "when user is not authenticated" do
    it "renders navigation with login link" do
      # Set up component according to TDD test case
      component = Components::Layouts::Navigation.new(current_user: nil)
      
      # Stub URL helpers to isolate test from Rails routing
      stub_url_helpers(component)
      
      # We need to stub form and link helpers
      allow(component).to receive(:link_to) do |text, path|
        "<a href=\"#{path}\">#{text}</a>"
      end
      
      allow(component).to receive(:form_with) do |**kwargs, &block|
        form_content = block.call if block
        "<form action=\"#{kwargs[:url]}\" method=\"#{kwargs[:method] || 'post'}\">#{form_content}</form>"
      end
      
      # Use the Phlex testing approach
      render(component)
      
      # Test the rendered output for unauthenticated experience
      expect(rendered).to include("Medicines")
      expect(rendered).to include("People")
      expect(rendered).to include("Medicine Finder")
      expect(rendered).to include("Login")
      expect(rendered).not_to include("Sign out")
    end
  end
end
