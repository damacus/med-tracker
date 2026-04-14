# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::DialogMiddle, type: :component do
  it 'uses token-driven surfaces instead of literal hex fills' do
    rendered = render_inline(described_class.new { 'Body' })
    html = rendered.to_html

    expect(html).not_to include('#')
    expect(html).not_to include('bg-[#')
  end
end
