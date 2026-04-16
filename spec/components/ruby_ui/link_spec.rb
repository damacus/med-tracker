# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Link, type: :component do
  it 'maps filled to the primary link styles' do
    rendered = render_inline(described_class.new(href: '/test', variant: :filled)) { 'Filled' }

    link = rendered.at_css('a')

    expect(link[:class]).to include('bg-primary')
    expect(link[:class]).to include('text-primary-foreground')
  end

  it 'maps outlined to the outline link styles' do
    rendered = render_inline(described_class.new(href: '/test', variant: :outlined)) { 'Outlined' }

    link = rendered.at_css('a')

    expect(link[:class]).to include('border')
    expect(link[:class]).to include('border-outline')
  end

  it 'maps text to the ghost link styles' do
    rendered = render_inline(described_class.new(href: '/test', variant: :text)) { 'Text' }

    link = rendered.at_css('a')

    expect(link[:class]).to include('hover:bg-tertiary-container')
    expect(link[:class]).to include('no-underline')
  end
end
