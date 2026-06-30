# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Locations::IndexView, type: :component do
  fixtures :locations

  it 'renders location card actions with shared M3 sizing and shape', :aggregate_failures do
    rendered = render_inline(described_class.new(locations: [locations(:home)]))
    action_elements = rendered.css('a, button').select do |element|
      element.text.match?(/View/) || element['aria_label'].present?
    end
    action_classes = action_elements.map { |element| element[:class].split }

    expect(action_classes).not_to be_empty
    expect(action_classes).to all(include_touch_target_class)
    expect(action_classes).to all(include('rounded-shape-full'))
    expect(action_classes.flatten).not_to include('rounded-xl')
    expect(action_classes.flatten).not_to include('w-10')
    expect(action_classes.flatten).not_to include('h-10')
  end

  def include_touch_target_class
    satisfy { |classes| classes.include?('min-h-11') || classes.include?('min-h-[44px]') }
  end
end
