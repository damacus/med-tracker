# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::SelectContent, type: :component do
  it 'uses the shared popover surface instead of the tinted page background' do
    rendered = render_inline(described_class.new { 'Option list' })
    html = rendered.to_html

    expect(html).to include('bg-popover')
    expect(html).not_to include('bg-background')
  end
end
