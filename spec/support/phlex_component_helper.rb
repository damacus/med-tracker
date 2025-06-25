# frozen_string_literal: true

module PhlexComponentHelper
  def render_inline(component)
    rendered = component.call
    Capybara::Node::Simple.new(rendered)
  end
end

RSpec.configure do |config|
  config.include PhlexComponentHelper, type: :component
end
