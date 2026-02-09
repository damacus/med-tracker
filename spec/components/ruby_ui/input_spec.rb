# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Input, type: :component do
  it 'includes min-h-[36px] to match Button :md height' do
    rendered = render_inline(described_class.new)

    input = rendered.css('input').first
    expect(input['class']).to include('min-h-[36px]')
  end

  it 'renders with h-9 base height' do
    rendered = render_inline(described_class.new)

    input = rendered.css('input').first
    expect(input['class']).to include('h-9')
  end
end
