# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::BreadcrumbLink, type: :component do
  it 'renders an anchor tag' do
    rendered = render_inline(described_class.new(href: '/dashboard')) { 'Dashboard' }

    link = rendered.css('a').first
    expect(link[:href]).to eq('/dashboard')
    expect(rendered.css('a').inner_html).to include('Dashboard')
  end
end
