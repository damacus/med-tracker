# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::DialogContent, type: :component do
  it 'uses token-driven floating surfaces instead of hard-coded white' do
    rendered = render_inline(described_class.new(size: :md) { 'Dialog body' })
    html = rendered.to_html

    expect(html).to include('bg-popover')
    expect(html).to include('bg-foreground/10')
    expect(html).to include('backdrop-blur-[1.5px]')
    expect(html).not_to include('bg-white')
    expect(html).not_to include('bg-background/80')
  end
end
