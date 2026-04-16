# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::DialogHeader, type: :component do
  it 'uses token-driven surfaces instead of literal gradient fills' do
    rendered = render_inline(described_class.new { 'Header' })
    html = rendered.to_html

    expect(html).not_to include('bg-gradient-to-b')
    expect(html).not_to include('#')
  end
end
