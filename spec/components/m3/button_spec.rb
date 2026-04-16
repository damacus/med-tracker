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

  it 'renders semantic outline variants with M3 base classes' do
    destructive = render_inline(described_class.new(variant: :destructive_outline) { 'Delete' })
    success = render_inline(described_class.new(variant: :success_outline) { 'Activate' })

    [destructive, success].each do |rendered|
      expect(rendered.to_html).to include('rounded-shape-full')
      expect(rendered.to_html).to include('state-layer')
    end

    expect(destructive.to_html).to include('text-error')
    expect(success.to_html).to include('text-success')
  end
end
