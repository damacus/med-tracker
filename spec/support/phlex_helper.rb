# frozen_string_literal: true

require "rails_helper"

# This module follows the approach from https://www.phlex.fun/components/testing.html
module PhlexTestingSupport
  # The basic render helper from the Phlex docs
  def render(component)
    @rendered = component.call.to_s
  end

  # Access to the rendered content
  def rendered
    @rendered
  end

  # Stub common URL helpers for isolated component tests
  def stub_url_helpers(component)
    # Define the paths needed for navigation component
    paths = {
      medicines_path: "/medicines",
      people_path: "/people",
      medicine_finder_path: "/medicine_finder",
      login_path: "/login",
      session_path: "/session"
    }
    
    # Apply stubs to the component
    paths.each do |method_name, path|
      allow(component).to receive(method_name).and_return(path)
    end
  end
  
  # Create a test controller with request context
  def controller
    @controller ||= ApplicationController.new.tap do |controller|
      request = ActionDispatch::TestRequest.create
      controller.instance_variable_set(:@_request, request)
      controller.request = request
    end
  end
  
  # You may need this for form helpers in components
  def setup_form_helpers(component)
    if component.respond_to?(:controller=)
      component.controller = controller
    end
  end
end

RSpec.configure do |config|
  # Include the helpers in component and view tests
  config.include PhlexTestingSupport, type: :component
  config.include PhlexTestingSupport, type: :view
end
