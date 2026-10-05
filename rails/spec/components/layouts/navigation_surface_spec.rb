# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Navigation, type: :component do
  it 'uses a solid token-driven mobile top bar surface' do
    rendered = render_inline(described_class.new(current_user: nil))
    html = rendered.to_html

    expect(html).not_to include('bg-card/95')
    expect(html).not_to include('bg-card/60')
    expect(html).not_to include('backdrop-blur')
    expect(html).to include('border-b border-outline-variant bg-surface-container shadow-elevation-1 md:hidden')
  end
end
