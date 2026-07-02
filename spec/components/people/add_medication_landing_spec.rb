# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::People::AddMedicationLanding, type: :component do
  it 'renders workflow options with the shared M3 link contract' do
    person = create(:person, name: 'Alex Patient')

    rendered = render_inline(described_class.new(person: person, back_path: add_medication_path))
    options = rendered.css("a[data-turbo-frame='modal']").select do |link|
      link.text.match?(/Prescribed|As needed/)
    end

    expect(options.size).to eq(2)
    expect(options.map { |option| option['class'].split }).to all(include('state-layer', 'rounded-2xl'))
    expect(options.map { |option| option['class'].split }).to all(include_touch_target_class)
  end

  it 'renders the back action as a shared text link' do
    person = create(:person, name: 'Alex Patient')

    rendered = render_inline(described_class.new(person: person, back_path: add_medication_path))
    back_link = rendered.at_css("a[href='#{add_medication_path}']")
    classes = back_link['class'].split

    expect(back_link.text).to include('Back')
    expect(classes).to include('state-layer')
    expect(classes).not_to include('rounded-xl')
  end

  def include_touch_target_class
    satisfy { |classes| classes.include?('min-h-11') || classes.include?('min-h-[44px]') }
  end

  def add_medication_path
    Rails.application.routes.url_helpers.add_medication_path(household_slug: 'test-household')
  end
end
