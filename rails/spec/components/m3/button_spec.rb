# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::M3::Button, type: :component do
  it 'renders a labelled button with a minimum touch target' do
    rendered = render_inline(described_class.new { 'Click me' })
    button = rendered.at_css('button')

    expect(button.text).to eq('Click me')
    expect(button[:class].split).to include('min-h-[44px]')
  end
end
