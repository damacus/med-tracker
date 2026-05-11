# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Icons::Medication, type: :component do
  let(:icon_path) do
    [
      'M420-260h120v-100h100v-120H540v-100H420v100H320v120h100v100Z',
      'M280-120q-33 0-56.5-23.5T200-200v-440q0-33 23.5-56.5T280-720h400',
      'q33 0 56.5 23.5T760-640v440q0 33-23.5 56.5T680-120H280Z',
      'm0-80h400v-440H280v440Zm-40-560v-80h480v80H240Zm40 120v440-440Z'
    ].join
  end

  it 'renders as a currentColor Material Symbols SVG' do
    rendered = render_inline(described_class.new(size: 24, class: 'text-primary'))
    svg = rendered.at_css('svg')

    expect(svg['viewbox']).to eq('0 -960 960 960')
    expect(svg['fill']).to eq('currentColor')
    expect(svg['stroke']).to be_nil
    expect(svg['class'].split).to include('material-symbol', 'material-symbol-medication', 'text-primary')
  end

  it 'renders the Material Symbols medication path' do
    rendered = render_inline(described_class.new)

    expect(rendered.css("path[d='#{icon_path}']").count).to eq(1)
  end
end
