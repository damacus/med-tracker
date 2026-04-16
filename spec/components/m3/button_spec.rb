# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::M3::Button, type: :component do
  it 'renders a filled button by default' do
    rendered = render_inline(described_class.new { 'Click me' })
    expect(rendered.to_html).to include('bg-primary')
    expect(rendered.to_html).to include('text-on-primary')
    expect(rendered.to_html).to include('state-layer')
    expect(rendered.to_html).to include('rounded-shape-full')
  end

  it 'renders a tonal button' do
    rendered = render_inline(described_class.new(variant: :tonal) { 'Tonal' })
    expect(rendered.to_html).to include('bg-secondary-container')
    expect(rendered.to_html).to include('text-on-secondary-container')
  end

  it 'renders an elevated button' do
    rendered = render_inline(described_class.new(variant: :elevated) { 'Elevated' })
    expect(rendered.to_html).to include('bg-surface-container-low')
    expect(rendered.to_html).to include('shadow-elevation-1')
  end

  it 'renders an outlined button' do
    rendered = render_inline(described_class.new(variant: :outlined) { 'Outlined' })
    expect(rendered.to_html).to include('border-outline')
  end

  it 'renders a text button' do
    rendered = render_inline(described_class.new(variant: :text) { 'Text' })
    expect(rendered.to_html).to include('bg-transparent')
  end
end
