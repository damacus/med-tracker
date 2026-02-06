# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Alert, type: :component do
  it 'renders with role alert' do
    rendered = render_inline(described_class.new)

    expect(rendered.css("div[role='alert']")).to be_present
  end

  it 'renders destructive variant with semantic classes' do
    rendered = render_inline(described_class.new(variant: :destructive))
    classes = rendered.css("div[role='alert']").first['class']

    expect(classes).to include('text-destructive-text')
    expect(classes).to include('bg-destructive/10')
    expect(classes).not_to include('text-red-900')
  end

  it 'renders warning variant with semantic classes' do
    rendered = render_inline(described_class.new(variant: :warning))
    classes = rendered.css("div[role='alert']").first['class']

    expect(classes).to include('text-warning-text')
    expect(classes).to include('bg-warning/10')
    expect(classes).not_to include('text-amber-900')
  end

  it 'renders success variant with semantic classes' do
    rendered = render_inline(described_class.new(variant: :success))
    classes = rendered.css("div[role='alert']").first['class']

    expect(classes).to include('text-success-text')
    expect(classes).to include('bg-success/10')
    expect(classes).not_to include('text-green-900')
  end

  it 'merges custom classes' do
    rendered = render_inline(described_class.new(class: 'custom-class'))

    expect(rendered.css('.custom-class')).to be_present
  end
end
