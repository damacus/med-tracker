# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Breadcrumbs, type: :component do
  it 'renders a navigation with breadcrumb aria-label' do
    rendered = render_inline(described_class.new)

    expect(rendered.css('nav[aria-label="breadcrumb"]')).to be_present
    expect(rendered.css('ol')).to be_present
  end
end
