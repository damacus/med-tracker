# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Sidebar, type: :component do
  fixtures :accounts, :people, :users

  let(:admin_user) { users(:admin) }
  let(:carer_user) { users(:carer) }
  let(:household) { admin_user.person.household }
  let(:household_slug) { household.slug }
  let(:membership) do
    household.household_memberships.find_or_create_by!(account: admin_user.person.account) do |record|
      record.person = admin_user.person
      record.role = :owner
      record.status = :active
    end
  end

  before do
    Current.household = household
    Current.membership = membership
  end

  after do
    Current.reset
  end

  def render_sidebar(user: nil, path: '/')
    vc = view_context
    vc.singleton_class.define_method(:current_user) { user }
    allow(vc.request).to receive(:path).and_return(path)

    html = vc.render(described_class.new(current_user: user))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  context 'when user is authenticated' do
    it 'renders the main sidebar content' do
      rendered = render_sidebar(user: admin_user)

      expect(rendered.text).to include(
        'MedTracker',
        'Dashboard',
        'Inventory',
        'Reports',
        'Administration',
        admin_user.person.name,
        'Owner',
        'Sign Out'
      )
    end

    it 'uses a readable active state for the highlighted link' do
      inventory_path = Rails.application.routes.url_helpers.medications_path(household_slug: household_slug)
      rendered = render_sidebar(user: admin_user, path: inventory_path)
      inventory_link = rendered.at_css(%(a[href="#{inventory_path}"]))

      expect(inventory_link['class']).to include('bg-secondary-container')
      expect(inventory_link['class']).to include('text-on-secondary-container')
    end

    it 'renders the shared person avatar in the profile navigation item' do
      rendered = render_sidebar(user: admin_user)
      profile_path = Rails.application.routes.url_helpers.profile_path(household_slug: household_slug)
      profile_link = rendered.at_css(%(a[href="#{profile_path}"]))

      expect(profile_link.at_css('[data-testid="person-avatar"]')).to be_present
    end
  end

  context 'when user is not an administrator' do
    it 'does not render Administration link' do
      Current.membership = household.household_memberships.find_or_create_by!(
        account: carer_user.person.account
      ) do |record|
        record.person = carer_user.person
        record.role = :member
        record.status = :active
      end
      rendered = render_sidebar(user: carer_user)

      expect(rendered.text).not_to include('Administration')
    end
  end

  context 'when user is not authenticated' do
    it 'renders nothing' do
      rendered = render_sidebar(user: nil)
      expect(rendered.text.strip).to be_empty
    end
  end
end
