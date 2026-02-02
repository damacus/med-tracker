# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Flash, type: :component do
  it 'renders flash container above navigation' do
    rendered = render_inline(described_class.new(notice: 'Saved'))

    expect(rendered.to_html).to include('fixed')
    expect(rendered.to_html).to include('top-4')
    expect(rendered.to_html).to include('z-[60]')
  end
end
