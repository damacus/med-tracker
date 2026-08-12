# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Navigation, type: :component do
  fixtures :accounts, :households, :people, :users

  after { Current.reset }

  describe 'i18n translations' do
    it 'renders navigation with default locale translations' do
      component = described_class.new(current_user: nil)

      rendered = render_inline(component)

      expect(rendered.to_html).to include('MedTracker')
      expect(rendered.to_html).to include('Skip to content')
      expect(rendered.to_html).to include('Login')
    end
  end

  describe 'brand readability' do
    it 'uses foreground text color utility for the brand link' do
      component = described_class.new(current_user: nil)

      rendered = render_inline(component)

      expect(rendered.to_html).to include('nav__brand-link text-foreground')
    end
  end

  describe 'accessibility' do
    it 'hides the search icon from the labelled search button' do
      rendered = render_inline(described_class.new(current_user: users(:admin)))

      search_button = rendered.at_css(%(button[aria-label="#{I18n.t('global_search.open')}"]))

      expect(search_button).to be_present
      expect(search_button.at_css('svg[aria-hidden="true"]')).to be_present
    end
  end

  it 'uses the preloaded shell context without queries or Current mutation' do
    user = users(:admin)
    account = user.person.account
    household = households(:fixture_household)
    membership = HouseholdMembership.find_or_initialize_by(account: account, household: household)
    membership.update!(person: user.person, role: :owner, status: :active)
    Current.account = account
    current_state = [Current.account, Current.household, Current.membership]

    rendered = nil
    query_count = count_household_queries do
      rendered = render_inline(described_class.new(current_user: user, membership: membership, household: household))
    end

    expect(query_count).to eq(0)
    expect(rendered.to_html).to include("/households/#{household.slug}/dashboard")
    expect(rendered.to_html).to include("/households/#{household.slug}/admin")
    expect([Current.account, Current.household, Current.membership]).to eq(current_state)
  end

  def count_household_queries(&)
    count = 0
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      count += 1 if payload[:sql].match?(/household_memberships|households/)
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end
end
