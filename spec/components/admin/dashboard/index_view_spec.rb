# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Dashboard::IndexView, type: :component do
  subject(:dashboard_view) { described_class.new }

  it 'renders quick action cards using RubyUI styling' do
    rendered = render_inline(dashboard_view)

    dashboard = rendered.css('[data-testid="admin-dashboard"]').first
    expect(dashboard).to be_present

    cards = dashboard.css('.bg-background.shadow')
    expect(cards.count).to eq(2)

    card_text = cards.map(&:text).join(' ')
    expect(card_text).to include('User Management')
    expect(card_text).to include('Audit Trail')
  end
end
