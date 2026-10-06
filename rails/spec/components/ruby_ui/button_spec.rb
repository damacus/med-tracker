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

    expect(rendered.css('.bg-error')).to be_present
    expect(rendered.css('.text-on-error')).to be_present
  end

  it 'renders outline variant with border' do
    rendered = render_inline(described_class.new(variant: :outline)) { 'Cancel' }

    expect(rendered.css('.border')).to be_present
    expect(rendered.css('.bg-background')).to be_present
  end

  it 'renders destructive_outline variant with border and semantic destructive text' do
    rendered = render_inline(described_class.new(variant: :destructive_outline)) { 'Delete' }

    button = rendered.css('button').first
    classes = button['class']

    expect(classes).to include('border')
    expect(classes).to include('text-error')
    expect(classes).to include('hover:bg-error-container')
  end

  it 'renders success_outline variant with border and semantic success text' do
    rendered = render_inline(described_class.new(variant: :success_outline)) { 'Activate' }

    button = rendered.css('button').first
    classes = button['class']

    expect(classes).to include('border')
    expect(classes).to include('text-success')
    expect(classes).to include('hover:bg-success-container')
  end

  it 'merges custom classes' do
    rendered = render_inline(described_class.new(class: 'custom-class')) { 'Custom' }

    expect(rendered.css('.custom-class')).to be_present
  end
end
