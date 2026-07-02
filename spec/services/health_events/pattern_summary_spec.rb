# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HealthEvents::PatternSummary do
  fixtures :people

  it 'returns no summaries when there are no repeated illness titles' do
    HealthEvent.create!(person: people(:john), event_kind: :illness, title: 'Cold', started_on: Date.new(2026, 1, 1))

    summaries = described_class.new(events: HealthEvent.illness).call

    expect(summaries).to be_empty
  end

  it 'ignores repeated illness episodes with blank titles' do
    events = [
      HealthEvent.new(event_kind: :illness, title: ' ', started_on: Date.new(2026, 1, 1)),
      HealthEvent.new(event_kind: :illness, title: nil, started_on: Date.new(2026, 2, 1))
    ]

    summaries = described_class.new(events: events).call

    expect(summaries).to be_empty
  end

  it 'returns no average duration when repeated episodes are ongoing' do
    create_illness_episode(Date.new(2026, 1, 1), nil, 'Migraine')
    create_illness_episode(Date.new(2026, 1, 10), nil, 'Migraine')

    summary = described_class.new(events: HealthEvent.illness).call.sole

    expect(summary.average_duration_days).to be_nil
  end

  it 'groups illness episodes by normalized title with factual interval and duration data' do
    create_illness_episode(Date.new(2026, 1, 1), Date.new(2026, 1, 3), '  Tonsillitis ')
    create_illness_episode(Date.new(2026, 2, 1), Date.new(2026, 2, 4), 'tonsillitis')
    create_illness_episode(Date.new(2026, 3, 1), nil, 'TONSILLITIS')

    summary = described_class.new(events: HealthEvent.illness).call.sole

    expect(summary).to have_attributes(
      normalized_title: 'tonsillitis',
      episode_count: 3,
      average_duration_days: 4,
      average_interval_days: 30,
      first_started_on: Date.new(2026, 1, 1),
      most_recent_started_on: Date.new(2026, 3, 1)
    )
  end

  def create_illness_episode(started_on, ended_on, title)
    HealthEvent.create!(
      person: people(:john),
      event_kind: :illness,
      title: title,
      started_on: started_on,
      ended_on: ended_on
    )
  end
end
