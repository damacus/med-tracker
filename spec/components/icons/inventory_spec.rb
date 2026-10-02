# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Icons::Inventory, type: :component do
  it 'renders as a currentColor Material Symbols SVG' do
    rendered = render_inline(described_class.new(size: 24, class: 'text-primary'))
    svg = rendered.at_css('svg')

    expect(svg['viewbox']).to eq('0 -960 960 960')
    expect(svg['fill']).to eq('currentColor')
    expect(svg['stroke']).to be_nil
    expect(svg['class'].split).to include('material-symbol', 'material-symbol-inventory', 'text-primary')
  end

  it 'preserves caller attributes' do
    rendered = render_inline(described_class.new(size: 24, class: 'text-primary'))
    svg = rendered.at_css('svg')

    expect(svg['width']).to eq('24')
  end
end
