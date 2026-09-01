# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::GpDateRange do
  it 'defaults to the same date twelve months earlier' do
    travel_to Time.zone.local(2026, 7, 9, 12) do
      range = described_class.parse(start_date: nil, end_date: nil)

      expect(range.start_date).to eq(Date.new(2025, 7, 9))
      expect(range.end_date).to eq(Date.new(2026, 7, 9))
    end
  end

  it 'accepts a 366-day range and rejects a longer one' do
    expect(described_class.parse(start_date: '2024-01-01', end_date: '2025-01-01')).to be_a(described_class)

    expect do
      described_class.parse(start_date: '2024-01-01', end_date: '2025-01-02')
    end.to raise_error(described_class::RangeTooLarge)
  end
end
