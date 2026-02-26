# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::QuickActions, type: :component do
  it 'renders heading and action links using Link component' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Quick Actions')
    links = rendered.css('a')
    expect(links).not_to be_empty
    expect(links.first.text).to include('Add Medication')
  end
end
