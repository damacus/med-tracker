# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Locations::FormView, type: :component do
  let(:location) { build(:location, name: 'Medicine cabinet', description: 'Hallway cupboard') }

  it 'renders the form with M3 surface, typography, and accessible field labels' do
    rendered = render_inline(
      described_class.new(
        location: location,
        title: 'New Location',
        subtitle: 'Add a new medication storage location'
      )
    )

    expect(rendered.at_css('form[data-testid="location-form"]')).to be_present
    expect(rendered.at_css('label[for="location_name"]').text).to eq('Name')
    expect(rendered.at_css('label[for="location_description"]').text).to include('Description')

    expect(rendered).to have_m3_location_fields
    expect(rendered.to_html).not_to include('rounded-[2.5rem]')
  end

  def have_m3_location_fields
    satisfy do |html|
      %w[location_name location_description].all? do |field_id|
        html.at_css("##{field_id}")['class'].include?('bg-surface-container-lowest')
      end
    end
  end
end
