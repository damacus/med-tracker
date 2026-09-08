# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::BreadcrumbSeparator, type: :component do
  it 'renders a chevron right icon by default' do
    rendered = render_inline(described_class.new)

    li = rendered.css('li[role="presentation"][aria-hidden="true"]').first
    expect(li).to be_present
    expect(rendered.to_html).to include('m9 18 6-6-6-6')
  end

  it 'yields a custom block if provided' do
    rendered = render_inline(described_class.new) { '/' }

    expect(rendered.to_html).to include('/')
  end
end
