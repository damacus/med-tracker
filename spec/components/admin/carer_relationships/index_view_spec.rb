# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::CarerRelationships::IndexView, type: :component do
  fixtures :accounts, :people, :users, :carer_relationships

  it 'renders mobile relationship cards and keeps the desktop table' do
    rendered = render_inline(described_class.new(relationships: [carer_relationships(:jane_cares_for_child)]))

    expect(rendered.css('[data-testid="admin-carer-relationships-mobile-list"]')).to be_present
    expect(rendered.css('[data-testid="admin-carer-relationships-desktop-table"] table')).to be_present
    expect(rendered.css('[data-testid="admin-carer-relationships-mobile-list"]').text).to include('Jane Doe')
    expect(rendered.css('[data-testid="admin-carer-relationships-mobile-list"]').text).to include('Child Patient')
  end
end
