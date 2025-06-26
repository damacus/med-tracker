# frozen_string_literal: true

require "rails_helper"

# This module follows the approach from https://www.phlex.fun/components/testing.html
module PhlexTestingSupport
  # Basic render method for components
  def render(component)
    @rendered = component.call.to_s
  end

  # Enhanced render_inline helper for nested components
  def render_inline(component)
    # Set component's controller if it accepts one
    component.controller = controller if component.respond_to?(:controller=)

    # Render using Rails view context
    component.instance_variable_set(:@view_context, view_context) if component.respond_to?(:view_context)

    # Create the output
    html = component.call.to_s
    @rendered = html
    
    Nokogiri::HTML::DocumentFragment.parse(html)
  end
  
  # Properly delegate rendering through Rails view context
  def view_render(...)
    view_context.render(...)
  end

  # Access to the rendered content
  def rendered
    @rendered
  end
  
  # Return a proper Rails view context
  def view_context
    controller.view_context
  end
  
  # Use Rails test controller for proper testing environment
  def controller
    @controller ||= ActionView::TestCase::TestController.new.tap do |controller|
      # Set up default URL options
      controller.instance_variable_set(:@_routes, Rails.application.routes)
      controller.singleton_class.include(Rails.application.routes.url_helpers)
    end
  end
end

RSpec.configure do |config|
  # Include the helpers in component and view tests
  config.include PhlexTestingSupport, type: :component
  config.include PhlexTestingSupport, type: :view
end
