# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::SelectItem, type: :component do
  it 'renders the check icon at size 16 using numeric sizing' do
    rendered = render_inline(described_class.new(value: 'test')) { 'Option' }

    svg = rendered.css('svg').first
    expect(svg['width']).to eq('16')
    expect(svg['height']).to eq('16')
    expect(svg['class']).not_to include('h-4')
    expect(svg['class']).not_to include('w-4')
  end
end
