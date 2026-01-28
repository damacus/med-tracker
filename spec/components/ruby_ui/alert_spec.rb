# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Alert, type: :component do
  it 'renders with role alert' do
    rendered = render_inline(described_class.new)

    expect(rendered.css("div[role='alert']")).to be_present
  end

  it 'renders destructive variant with correct classes' do
    rendered = render_inline(described_class.new(variant: :destructive))

    expect(rendered.css('.text-red-900')).to be_present
  end

  it 'renders warning variant with correct classes' do
    rendered = render_inline(described_class.new(variant: :warning))

    expect(rendered.css('.text-amber-900')).to be_present
  end

  it 'renders success variant with correct classes' do
    rendered = render_inline(described_class.new(variant: :success))

    expect(rendered.css('.text-green-900')).to be_present
  end

  it 'merges custom classes' do
    rendered = render_inline(described_class.new(class: 'custom-class'))

    expect(rendered.css('.custom-class')).to be_present
  end
end
