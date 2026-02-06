# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Button, type: :component do
  it 'renders a button element' do
    rendered = render_inline(described_class.new) { 'Click me' }

    expect(rendered.css('button')).to be_present
  end

  it 'renders primary variant by default' do
    rendered = render_inline(described_class.new) { 'Primary' }

    expect(rendered.css('.bg-primary')).to be_present
  end

  it 'renders destructive variant with filled background' do
    rendered = render_inline(described_class.new(variant: :destructive)) { 'Delete' }

    expect(rendered.css('.bg-destructive')).to be_present
    expect(rendered.css('.text-white')).to be_present
  end

  it 'renders outline variant with border' do
    rendered = render_inline(described_class.new(variant: :outline)) { 'Cancel' }

    expect(rendered.css('.border')).to be_present
    expect(rendered.css('.bg-background')).to be_present
  end

  it 'renders destructive_outline variant with border and red text' do
    rendered = render_inline(described_class.new(variant: :destructive_outline)) { 'Delete' }

    button = rendered.css('button').first
    classes = button['class']

    expect(classes).to include('border')
    expect(classes).to include('text-red-600')
    expect(classes).to include('hover:bg-red-50')
    expect(classes).to include('hover:text-red-700')
    expect(classes).not_to include('bg-destructive')
  end

  it 'renders success_outline variant with border and green text' do
    rendered = render_inline(described_class.new(variant: :success_outline)) { 'Activate' }

    button = rendered.css('button').first
    classes = button['class']

    expect(classes).to include('border')
    expect(classes).to include('text-green-600')
    expect(classes).to include('hover:bg-green-50')
    expect(classes).to include('hover:text-green-700')
    expect(classes).not_to include('bg-primary')
  end

  it 'merges custom classes' do
    rendered = render_inline(described_class.new(class: 'custom-class')) { 'Custom' }

    expect(rendered.css('.custom-class')).to be_present
  end
end
