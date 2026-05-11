# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Icons::Inventory, type: :component do
  let(:icon_path) do
    [
      'M620-163 450-333l56-56 114 114 226-226 56 56-282 282Z',
      'm220-397h-80v-200h-80v120H280v-120h-80v560h240v80H200',
      'q-33 0-56.5-23.5T120-200v-560q0-33 23.5-56.5T200-840h167',
      'q11-35 43-57.5t70-22.5q40 0 71.5 22.5T594-840h166',
      'q33 0 56.5 23.5T840-760v200ZM480-760q17 0 28.5-11.5T520-800',
      'q0-17-11.5-28.5T480-840q-17 0-28.5 11.5T440-800',
      'q0 17 11.5 28.5T480-760Z'
    ].join
  end

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

  it 'renders the Material Symbols inventory path' do
    rendered = render_inline(described_class.new)

    expect(rendered.css("path[d='#{icon_path}']").count).to eq(1)
  end
end
