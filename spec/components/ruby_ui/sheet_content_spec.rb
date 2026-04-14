# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::SheetContent, type: :component do
  it 'uses token-driven shell surfaces for the drawer' do
    rendered = render_inline(described_class.new(side: :right) { 'Sheet body' })
    html = rendered.to_html

    expect(html).to include('bg-foreground/10')
    expect(html).to include('backdrop-blur-[1.5px]')
    expect(html).to include('bg-popover')
    expect(html).not_to include('bg-slate-950/12')
  end
end
