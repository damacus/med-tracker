# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::BreadcrumbEllipsis, type: :component do
  it 'renders a more-horizontal icon by default' do
    rendered = render_inline(described_class.new)

    span = rendered.css('span[role="presentation"][aria-hidden="true"]').first
    expect(span).to be_present
    expect(rendered.css('svg').to_html).to include('circle cx="12" cy="12" r="1"')
    expect(rendered.css('.sr-only').inner_html).to eq('More')
  end

  it 'renders the icon at size 16 using numeric sizing' do
    rendered = render_inline(described_class.new)

    svg = rendered.css('svg').first
    expect(svg['width']).to eq('16')
    expect(svg['height']).to eq('16')
    expect(svg['class']).not_to include('h-4')
    expect(svg['class']).not_to include('w-4')
  end

  it 'yields a custom block if provided' do
    rendered = render_inline(described_class.new) { 'custom-ellipsis' }

    expect(rendered.css('span[role="presentation"]').inner_html).to include('custom-ellipsis')
  end
end
