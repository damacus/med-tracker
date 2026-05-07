# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::BottomNav, type: :component do
  it 'renders navigation links with 44px minimum touch targets' do
    rendered = render_inline(described_class.new)

    nav_links = rendered.css('a')
    nav_links.each do |link|
      classes = link['class'] || ''
      expect(classes).to include('min-h-[44px]'),
                         "Nav link '#{link.text.strip}' missing min-h-[44px] for WCAG 2.2 SC 2.5.8"
      expect(classes).to include('min-w-[44px]'),
                         "Nav link '#{link.text.strip}' missing min-w-[44px] for WCAG 2.2 SC 2.5.8"
    end
  end

  it 'uses stable icon containers to avoid active-state layout shift on mobile' do
    rendered = render_inline(described_class.new)

    rendered.css('a').each do |link|
      icon_container = link.css('div').first
      classes = icon_container['class'] || ''

      expect(classes).to include('h-8')
      expect(classes).to include('w-16')
      expect(classes).to include('shrink-0')
    end
  end
end
