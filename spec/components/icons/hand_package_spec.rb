# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Icons::HandPackage, type: :component do
  let(:icon_path) do
    [
      'M600-800H360v280h240v-280Zm200 0H680v280h120v-280Z',
      'M575-440H320v240h222q21 0 40.5-7t35.5-21l166-137q-8-8-18-12t-21-6',
      'q-17-3-33 1t-30 15l-108 87H400v-80h146l44-36q5-3 7.5-8t2.5-11',
      'q0-10-7.5-17.5T575-440Zm-335 0h-80v280h80v-280Z',
      'm40 0v-360q0-33 23.5-56.5T360-880h440q33 0 56.5 23.5T880-800v280',
      'q0 33-23.5 56.5T800-440H280ZM240-80h-80q-33 0-56.5-23.5T80-160v-280',
      'q0-33 23.5-56.5T160-520h415q85 0 164 29t127 98l27 41-223 186',
      'q-27 23-60 34.5T542-120H309q-11 18-29 29t-40 11Z'
    ].join
  end

  it 'renders as a currentColor Material Symbols SVG' do
    rendered = render_inline(described_class.new(size: 20, class: 'mr-2', aria_hidden: 'true'))
    svg = rendered.at_css('svg')

    expect(svg['viewbox']).to eq('0 -960 960 960')
    expect(svg['fill']).to eq('currentColor')
    expect(svg['stroke']).to be_nil
    expect(svg['class'].split).to include('material-symbol', 'material-symbol-hand-package', 'mr-2')
  end

  it 'preserves caller attributes' do
    rendered = render_inline(described_class.new(size: 20, class: 'mr-2', aria_hidden: 'true'))
    svg = rendered.at_css('svg')

    expect(svg['aria-hidden']).to eq('true')
    expect(svg['width']).to eq('20')
  end

  it 'renders the Material Symbols hand_package path' do
    rendered = render_inline(described_class.new)

    expect(rendered.css("path[d='#{icon_path}']").count).to eq(1)
  end
end
