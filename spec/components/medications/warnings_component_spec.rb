# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::WarningsComponent, type: :component do
  it 'renders safety warnings when present' do
    medication = create(:medication, warnings: 'Take with food')

    rendered = render_inline(described_class.new(medication: medication))

    expect(rendered.text).to include('Safety Warnings')
    expect(rendered.text).to include('Take with food')
  end
end
