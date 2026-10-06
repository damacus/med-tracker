# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::GlobalSearch::Palette, type: :component do
  fixtures :accounts, :households, :people, :users

  after { Current.reset }

  it 'hides the close icon from the labelled close button' do
    rendered = render_inline(described_class.new)
    close_button = rendered.at_css('button[aria-label="Close search"]')

    expect(close_button).to be_present
    expect(close_button.at_css('svg[aria-hidden="true"]')).to be_present
  end

  it 'uses the existing status target as a visually hidden polite live region' do
    rendered = render_inline(described_class.new)
    status = rendered.at_css('[data-global-search-target="status"]')

    expect(status['aria-live']).to eq('polite')
    expect(status['class'].split).to include('sr-only')
  end

  it 'uses the preloaded household without queries or Current mutation' do
    account = accounts(:admin)
    household = households(:fixture_household)
    Current.account = account
    current_state = [Current.account, Current.household, Current.membership]

    rendered = nil
    query_count = count_queries do
      rendered = render_inline(described_class.new(household: household))
    end

    expect(query_count).to eq(0)
    expect(rendered.at_css('#global_search_panel')['data-search-url'])
      .to include("/households/#{household.slug}/search.json")
    expect([Current.account, Current.household, Current.membership]).to eq(current_state)
  end

  def count_queries(&)
    count = 0
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      count += 1
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end
end
