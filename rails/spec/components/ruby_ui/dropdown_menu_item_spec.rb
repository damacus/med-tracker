# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::DropdownMenuItem, type: :component do
  it 'renders an anchor menu item by default' do
    rendered = render_inline(described_class.new(href: '/profile') { 'Profile' })

    item = rendered.at_css('a[role="menuitem"]')
    expect(item['href']).to eq('/profile')
    expect(item['data-action']).to eq('click->ruby-ui--dropdown-menu#close')
    expect(item['data-ruby-ui--dropdown-menu-target']).to eq('menuItem')
  end

  it 'renders a button menu item for form actions' do
    rendered = render_inline(described_class.new(as: :button, type: :submit) { 'Pause' })

    item = rendered.at_css('button[role="menuitem"]')
    expect(item['type']).to eq('submit')
    expect(item['data-action']).to eq('click->ruby-ui--dropdown-menu#close')
    expect(item['data-ruby-ui--dropdown-menu-target']).to eq('menuItem')
  end
end
