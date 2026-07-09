# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::DateRange do
  it 'defaults to the previous seven days ending today' do
    travel_to Time.zone.local(2026, 7, 9, 12) do
      range = described_class.parse(start_date: nil, end_date: nil)

      expect(range.start_date).to eq(Date.new(2026, 7, 3))
      expect(range.end_date).to eq(Date.new(2026, 7, 9))
    end
  end

  it 'parses supplied dates' do
    range = described_class.parse(start_date: '2026-01-01', end_date: '2026-01-31')

    expect(range.start_date).to eq(Date.new(2026, 1, 1))
    expect(range.end_date).to eq(Date.new(2026, 1, 31))
  end

  it 'rejects ranges longer than 180 days' do
    expect do
      described_class.parse(start_date: '2026-01-01', end_date: '2026-07-01')
    end.to raise_error(described_class::RangeTooLarge)
  end

  it 'rejects end dates before start dates' do
    expect do
      described_class.parse(start_date: '2026-02-01', end_date: '2026-01-01')
    end.to raise_error(ArgumentError)
  end
end
