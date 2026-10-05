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

    component = described_class.new(current_user: user)
    yield component if block_given?
    html = vc.render(component)
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  it 'renders only the three labelled quick links in order', :aggregate_failures do
    rendered = render_rail(user: admin_user)

    expect(rendered.css('aside[data-testid="mobile-rail"]')).to be_present
    expect(rendered.css('a').map { |link| link.text.strip }).to eq(['Home', 'Inventory', 'Medicine Finder'])
    expect(rendered.css('a').pluck('href')).to eq(
      %i[dashboard_path medications_path medication_finder_path].map do |route|
        Rails.application.routes.url_helpers.public_send(route, household_slug: household_slug)
      end
    )
    expect(rendered.css('button[aria-label="Sign Out"]')).to be_empty
    expect(rendered.css('a[aria-label] svg')).to all(satisfy { |icon| icon['aria-hidden'] == 'true' })
  end

  it 'does not render for an unauthenticated visitor' do
    expect(render_rail(user: nil).css('aside')).to be_empty
  end

  it 'renders the saved order and omits an administration shortcut without an admin membership' do
    Current.account = accounts(:jane_doe)
    Current.account.update!(mobile_shortcuts: %w[reports administration people])

    rendered = render_rail(user: admin_user)

    expect(rendered.css('a').map { |link| link.text.strip }).to eq(%w[Reports People])
  end

  it 'does not restore destinations excluded from the shared navigation' do
    rendered = render_rail(user: admin_user) do |component|
      allow(component).to receive(:primary_navigation_items).and_wrap_original do |method|
        method.call.reject { |item| item[:path].end_with?('/medication-finder') }
      end
    end

    expect(rendered.css('a').map { |link| link.text.strip }).to eq(%w[Home Inventory])
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

  it 'marks Home active on the dashboard alias route' do
    rendered = render_rail(
      user: admin_user,
      path: Rails.application.routes.url_helpers.dashboard_path(household_slug: household_slug)
    )
    dashboard_link = rendered.at_css(%(a[aria-label="Home"]))

    expect(dashboard_link['aria-current']).to eq('page')
  end
end
