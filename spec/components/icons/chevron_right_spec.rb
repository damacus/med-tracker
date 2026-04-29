# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Icons::ChevronRight, type: :component do
  it 'keeps the shared chevron defaults unchanged' do
    rendered = render_inline(described_class.new)
    svg = rendered.at_css('svg')

    expect(svg['stroke-width']).to eq('2')
    expect(rendered.css('path[d="m9 18 6-6-6-6"]').count).to eq(1)
  end

  it 'allows callers to opt into a custom chevron path' do
    rendered = render_inline(described_class.new(path: 'M9 5L16 12L9 19', stroke_width: '2.5'))
    svg = rendered.at_css('svg')

    expect(svg['stroke-width']).to eq('2.5')
    expect(rendered.css('path[d="M9 5L16 12L9 19"]').count).to eq(1)
  end
end
