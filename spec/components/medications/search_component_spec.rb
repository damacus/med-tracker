# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::SearchComponent, type: :component do
  let(:home) { create(:location, name: 'Search Home') }
  let(:school) { create(:location, name: 'Search School') }

  it 'renders category and location filters with the current selections' do
    rendered = render_inline(described_class.new(
                               current_category: 'Vitamin',
                               categories: %w[Analgesic Vitamin],
                               locations: [home, school],
                               current_location_id: school.id
                             ))

    expect(rendered.css("input[type='radio'][name='category'][value='Vitamin'][checked]")).to be_present
    expect(rendered.css("input[type='radio'][name='location_id'][value='#{school.id}'][checked]")).to be_present
    expect(rendered.text).to include('Category')
    expect(rendered.text).to include('Location')
  end

  it 'does not render anything when no filters are available' do
    rendered = render_inline(described_class.new(categories: [], locations: []))

    expect(rendered.text.strip).to eq('')
  end
end
