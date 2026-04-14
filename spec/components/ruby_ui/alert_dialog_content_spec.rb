# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::AlertDialogContent, type: :component do
  it 'uses token-driven floating surfaces instead of hard-coded white' do
    rendered = render_inline(described_class.new { 'Alert body' })
    html = rendered.to_html

    expect(html).to include('bg-popover')
    expect(html).to include('bg-foreground/10')
    expect(html).to include('backdrop-blur-[1.5px]')
    expect(html).not_to include('bg-white')
    expect(html).not_to include('bg-black/80')
  end
end
