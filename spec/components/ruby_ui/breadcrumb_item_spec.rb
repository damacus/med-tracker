# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::BreadcrumbItem, type: :component do
  it 'renders an inline list item' do
    rendered = render_inline(described_class.new) { 'Dashboard' }

    expect(rendered.css('li').inner_html).to include('Dashboard')
  end
end
