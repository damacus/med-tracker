# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medicines::ShowView, type: :component do
  let(:medicine) { create(:medicine, name: 'Paracetamol', current_supply: 50, stock: 100) }

  it 'renders the medicine name' do
    rendered = render_inline(described_class.new(medicine: medicine))

    expect(rendered.text).to include('Paracetamol')
  end

  it 'uses gap-2 for actions footer consistent with other card footers' do
    rendered = render_inline(described_class.new(medicine: medicine))

    actions_div = rendered.css('a').first&.parent
    expect(actions_div['class']).to include('gap-2')
    expect(actions_div['class']).not_to include('gap-3')
  end

  it 'does not define unused button class methods (signal-to-noise)' do
    view = described_class.new(medicine: medicine)

    expect(view.private_methods).not_to include(:button_primary_classes)
    expect(view.private_methods).not_to include(:button_secondary_classes)
  end

  it 'does not show both Current Supply and Stock cards (signal-to-noise)' do
    rendered = render_inline(described_class.new(medicine: medicine))

    headings = rendered.css('h2').map(&:text)
    supply_headings = headings.grep(/supply|stock/i)
    expect(supply_headings.length).to eq(1),
                                      "Expected 1 inventory heading but found #{supply_headings.length}: " \
                                      "#{supply_headings.inspect}. Redundant inventory cards violate " \
                                      'signal-to-noise ratio.'
  end
end
