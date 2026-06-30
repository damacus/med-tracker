# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::M3::Link, type: :component do
  it 'renders action links with the shared shape and touch target' do
    rendered = render_inline(described_class.new(href: '/test') { 'Open' })

    link = rendered.at_css('a')
    classes = link[:class].split

    expect(classes).to include('rounded-shape-full')
    expect(classes).to include('min-h-[44px]')
  end

  it 'renders outlined links with an M3 surface hover state' do
    rendered = render_inline(described_class.new(href: '/test', variant: :outlined) { 'Open' })

    expect(rendered.at_css('a')[:class]).to include('hover:bg-surface-container-low')
  end
end
