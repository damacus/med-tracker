# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Dashboard::QuickActions, type: :component do
  it 'does not define unused button_classes method (signal-to-noise)' do
    view = described_class.new

    expect(view.private_methods).not_to include(:button_classes)
  end

  it 'renders quick action links' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Quick Actions')
    expect(rendered.text).to include('Add Medicine')
  end
end
