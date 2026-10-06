# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationPausePeriods::IntervalProjection do
  it 'excludes only occurrences inside a completed partial-day pause' do
    period = build_period(started_at: time_at(9), ended_at: time_at(12))
    occurrences = [time_at(8), time_at(9), time_at(11), time_at(12), time_at(15)]

    active_occurrences = described_class.new(periods: [period]).active_occurrences(occurrences)

    expect(active_occurrences).to eq([time_at(8), time_at(12), time_at(15)])
  end

  it 'excludes every occurrence during a full-day pause' do
    period = build_period(started_at: time_at(0), ended_at: time_at(23, 59))

    active_occurrences = described_class.new(periods: [period]).active_occurrences([time_at(8), time_at(20)])

    expect(active_occurrences).to be_empty
  end

  it 'excludes occurrences after the start of a current open pause' do
    period = build_period(started_at: time_at(10), ended_at: nil)

    projection = described_class.new(periods: [period])

    expect(projection.paused_at?(time_at(9, 59))).to be(false)
    expect(projection.paused_at?(time_at(10))).to be(true)
    expect(projection.paused_at?(time_at(18))).to be(true)
  end

  it 'uses persistence time as the boundary for an unknown legacy start' do
    period = build_period(started_at: nil, ended_at: time_at(16), created_at: time_at(10))

    projection = described_class.new(periods: [period])

    expect(projection.paused_at?(time_at(9, 59))).to be(false)
    expect(projection.paused_at?(time_at(10))).to be(true)
    expect(projection.paused_at?(time_at(16))).to be(false)
  end

  def build_period(started_at:, ended_at:, created_at: time_at(7))
    MedicationPausePeriod.new(started_at:, ended_at:, created_at:)
  end

  def time_at(hour, minute = 0)
    Time.zone.local(2026, 5, 5, hour, minute)
  end
end
