# frozen_string_literal: true

require 'rails_helper'

module PhlexTestingSupport
  def render(component)
    @rendered = component.call.to_s
  end

  def render_inline(component)
    html = view_context.render(component)
    @rendered = html

    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def view_render(...)
    view_context.render(...)
  end

  def rendered
    @rendered
  end

  delegate :view_context, to: :controller

  def controller
    @controller ||= ActionView::TestCase::TestController.new.tap do |controller|
      controller.instance_variable_set(:@_routes, Rails.application.routes)
      controller.singleton_class.include(Rails.application.routes.url_helpers)
      controller.singleton_class.define_method(:default_url_options) do
        { host: 'test.host' }
      end
    end
  end
end

RSpec.configure do |config|
  config.include PhlexTestingSupport, type: :component
  config.include PhlexTestingSupport, type: :view
end
