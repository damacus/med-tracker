# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::IndexView, type: :component do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :schedules

  it 'renders mobile schedule cards and keeps the desktop table' do
    rendered = render_inline(described_class.new(schedules: [schedules(:john_paracetamol)]))

    expect(rendered.css('[data-testid="schedules-mobile-list"]')).to be_present
    expect(rendered.css('[data-testid="schedules-desktop-table"] table')).to be_present
    expect(rendered.css('[data-testid="schedules-mobile-list"]').text).to include('Paracetamol')
    expect(rendered.css('[data-testid="schedules-mobile-list"]').text).to include('John Doe')
  end
end
