# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::M3::Input, type: :component do
  it 'renders an M3 styled input' do
    rendered = render_inline(described_class.new(name: 'test', id: 'test'))
    expect(rendered.to_html).to include('rounded-shape-xs')
    expect(rendered.to_html).to include('border-outline')
    expect(rendered.to_html).to include('h-14')
  end
end
