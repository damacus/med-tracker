# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::MobileRail, type: :component do
  fixtures :accounts, :people, :users

  let(:admin_user) { users(:admin) }
  let(:household_slug) { 'test-household' }
  let(:household) { Household.find_or_create_by!(slug: household_slug) { |record| record.name = 'Test Household' } }

  before do
    Current.household = household
  end

  after do
    Current.reset
  end

  def render_rail(user:, path: '/')
    vc = view_context
    vc.singleton_class.define_method(:current_user) { user }
    allow(vc.request).to receive(:path).and_return(path)

    html = vc.render(described_class.new(current_user: user))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  it 'renders icon-only navigation with aria labels and no logout action', :aggregate_failures do
    rendered = render_rail(user: admin_user)

    expect(rendered.css('aside[data-testid="mobile-rail"]')).to be_present
    expect(rendered.css('a[aria-label="Dashboard"]')).to be_present
    expect(rendered.css('a[aria-label="Inventory"]')).to be_present
    expect(rendered.css('a[aria-label="Profile"]')).to be_present
    expect(rendered.css('button[aria-label="Sign Out"]')).to be_empty
    expect(rendered.css('a[aria-label] svg')).to all(satisfy { |icon| icon['aria-hidden'] == 'true' })
  end

  it 'renders the inventory item with the inventory icon' do
    rendered = render_rail(user: admin_user)

    expect(rendered.at_css('a[aria-label="Inventory"] svg.material-symbol-inventory')).to be_present
  end

  it 'marks the active rail item with aria-current' do
    rendered = render_rail(
      user: admin_user,
      path: Rails.application.routes.url_helpers.medications_path(household_slug: household_slug)
    )
    inventory_link = rendered.at_css(%(a[aria-label="Inventory"]))

    expect(inventory_link['aria-current']).to eq('page')
  end

  it 'marks Dashboard active on the dashboard alias route' do
    rendered = render_rail(
      user: admin_user,
      path: Rails.application.routes.url_helpers.dashboard_path(household_slug: household_slug)
    )
    dashboard_link = rendered.at_css(%(a[aria-label="Dashboard"]))

    expect(dashboard_link['aria-current']).to eq('page')
  end
end
