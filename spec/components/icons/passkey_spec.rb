# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Icons::Passkey, type: :component do
  let(:icon_path) do
    [
      'M144-192v-96q0-23 12.5-43.5T191-366q55-32 116.36-49T432-432',
      'q24 0 48 2.5t48 7.5q-1 46 19 87.5t55 71v71.5H144Z',
      'M750-72l-54-54.15V-294q-42-11-69-46t-27-80.19q0-54.61 38.72-93.21',
      'q38.72-38.6 93.5-38.6t93.28 38.66Q864-474.69 864-420',
      'q0 42.58-24.65 75.69Q814.7-311.19 777-297l45 45-54 54 54 54-72 72Z',
      'M432-480q-60 0-102-42t-42-102q0-60 42-102t102-42q60 0 102 42t42 102',
      'q0 60-42 102t-102 42Zm299.79 72q15.21 0 25.71-10.29t10.5-25.5',
      'q0-15.21-10.29-25.71t-25.5-10.5q-15.21 0-25.71 10.29t-10.5 25.5',
      'q0 15.21 10.29 25.71t25.5 10.5Z'
    ].join
  end

  it 'renders as a currentColor Material Symbols SVG' do
    rendered = render_inline(described_class.new(size: 24, class: 'text-primary'))
    svg = rendered.at_css('svg')

    expect(svg['viewbox']).to eq('0 -960 960 960')
    expect(svg['fill']).to eq('currentColor')
    expect(svg['stroke']).to be_nil
    expect(svg['class'].split).to include('material-symbol', 'material-symbol-passkey', 'text-primary')
  end

  it 'renders the Material Symbols passkey path' do
    rendered = render_inline(described_class.new)

    expect(rendered.css("path[d='#{icon_path}']").count).to eq(1)
  end
end
