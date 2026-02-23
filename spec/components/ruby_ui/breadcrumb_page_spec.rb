# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::BreadcrumbPage, type: :component do
  it 'renders a span with current=page aria attribute' do
    rendered = render_inline(described_class.new) { 'Current Page' }

    span = rendered.css('span').first
    expect(span['aria-disabled']).to eq('true')
    expect(span['aria-current']).to eq('page')
    expect(span['role']).to eq('link')
    expect(rendered.css('span').inner_html).to include('Current Page')
  end
end
