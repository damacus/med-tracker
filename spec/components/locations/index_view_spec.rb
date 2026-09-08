# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Locations::IndexView, type: :component do
  fixtures :locations, :medications

  it 'uses the dedicated medication count translation for one and many medications' do
    one_location = create(:location, household: locations(:home).household, name: 'One location')
    two_location = create(:location, household: locations(:home).household, name: 'Two location')
    one_medication = create(:medication, location: one_location, name: 'One medication')
    two_medications = create(:medication, location: two_location, name: 'First medication')
    create(:medication, location: two_location, name: 'Second medication')

    rendered = render_locations_index(locations: [one_medication.location, two_medications.location])

    expect(rendered.text).to include(I18n.t('locations.index.medication_count.one', count: 1))
    expect(rendered.text).to include(I18n.t('locations.index.medication_count.other', count: 2))
    expect(rendered.text).not_to include('Created')
  end

  it 'renders location card actions with shared touch targets', :aggregate_failures do
    rendered = render_locations_index
    action_elements = rendered.css('a, button').select do |element|
      element.text.match?(/View/) || element['aria_label'].present?
    end
    action_classes = action_elements.map { |element| element[:class].split }

    expect(action_classes).not_to be_empty
    expect(action_classes).to all(include_touch_target_class)
  end

  it 'renders the add location action for managers' do
    rendered = render_locations_index

    expect(rendered.at_css("a[href='#{view_context.new_location_path}']")).to be_present
  end

  it 'renders the shared responsive page header' do
    rendered = render_locations_index
    header = rendered.at_css('header')
    action = header.at_css("a[href='#{view_context.new_location_path}']")

    expect(header.at_css('h1').text).to include('Locations')
    expect(action).to be_present
  end

  it 'hides icons inside labelled location action controls', :aggregate_failures do
    rendered = render_locations_index

    edit_link = rendered.at_css('a[aria-label="Edit location"]')
    delete_button = rendered.at_css('button[aria-label="Delete location"]')

    expect(edit_link.at_css('svg[aria-hidden="true"]')).to be_present
    expect(delete_button.at_css('svg[aria-hidden="true"]')).to be_present
  end

  it 'renders only the view action for non-managers', :aggregate_failures do
    rendered = render_locations_index(create_allowed: false, update_allowed: false, destroy_allowed: false)

    expect(rendered.text).to include('View')
    expect(rendered.text).not_to include('Add Location')
    expect(rendered.at_css('a[aria-label="Edit location"]')).to be_nil
    expect(rendered.at_css('button[aria-label="Delete location"]')).to be_nil
  end

  def render_locations_index(
    locations: [locations(:home)], create_allowed: true, update_allowed: true, destroy_allowed: true
  )
    vc = view_context
    policy_stub = Struct.new(:create?, :update?, :destroy?).new(create_allowed, update_allowed, destroy_allowed)
    vc.singleton_class.define_method(:policy) { |_record| policy_stub }
    html = vc.render(described_class.new(locations: locations))

    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def include_touch_target_class
    satisfy { |classes| classes.include?('min-h-11') || classes.include?('min-h-[44px]') }
  end
end
