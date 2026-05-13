# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::CarerRelationships::IndexView, type: :component do
  fixtures :accounts, :people, :users, :carer_relationships

  let(:relationship) { carer_relationships(:jane_cares_for_child) }

  it 'renders mobile relationship cards and keeps the desktop table' do
    rendered = render_inline(described_class.new(relationships: [relationship]))

    expect(rendered.css('[data-testid="admin-carer-relationships-mobile-list"]')).to be_present
    expect(rendered.css('[data-testid="admin-carer-relationships-desktop-table"] table')).to be_present
    expect(rendered.css('[data-testid="admin-carer-relationships-mobile-list"]').text).to include('Jane Doe')
  end

  it 'keeps canonical row selectors unique when card representations render' do
    rendered = render_inline(described_class.new(relationships: [relationship]))

    expect(rendered.css('[data-testid="admin-carer-relationships-mobile-list"]').text).to include('Child Patient')
    expect(rendered.css("[data-relationship-id='#{relationship.id}']").length).to eq(1)
    expect(rendered.css("[data-relationship-card-id='#{relationship.id}']").text).to include('Deactivate')
  end
end
