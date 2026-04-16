# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::M3::Card, type: :component do
  it 'renders an elevated card by default' do
    rendered = render_inline(described_class.new { 'Content' })
    expect(rendered.to_html).to include('bg-surface-container-low')
    expect(rendered.to_html).to include('shadow-elevation-1')
    expect(rendered.to_html).to include('rounded-shape-xl')
  end

  it 'renders an outlined card' do
    rendered = render_inline(described_class.new(variant: :outlined) { 'Outlined' })
    expect(rendered.to_html).to include('bg-surface')
    expect(rendered.to_html).to include('border-outline')
  end

  it 'renders a filled card' do
    rendered = render_inline(described_class.new(variant: :filled) { 'Filled' })
    expect(rendered.to_html).to include('bg-surface-container')
  end
end
