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

    expect(classes).to include('text-on-error-container')
    expect(classes).to include('bg-error-container')
  end

  it 'renders warning variant with semantic classes' do
    rendered = render_inline(described_class.new(variant: :warning))
    classes = rendered.css("div[role='alert']").first['class']

    expect(classes).to include('text-on-warning-container')
    expect(classes).to include('bg-warning-container')
  end

  it 'renders success variant with semantic classes' do
    rendered = render_inline(described_class.new(variant: :success))
    classes = rendered.css("div[role='alert']").first['class']

    expect(classes).to include('text-on-success-container')
    expect(classes).to include('bg-success-container')
  end

  it 'merges custom classes' do
    rendered = render_inline(described_class.new(class: 'custom-class'))

    expect(rendered.css('.custom-class')).to be_present
  end
end
