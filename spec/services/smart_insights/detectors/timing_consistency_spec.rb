# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::TimingConsistency do
  # Build a Schedule double with a single configured time and always 1 expected dose per day.
  def schedule_double(id:, time:, start_date:, end_date:)
    sched = instance_double(
      Schedule,
      id: id,
      schedule_config: { 'times' => [time] }
    )
    (start_date..end_date).each do |date|
      allow(sched).to receive(:expected_doses_on).with(date).and_return(1)
    end
    sched
  end

  # Build a take double that lands exactly on the expected occurrence for a schedule + date.
  # The detector calls take.schedule to get schedule_config for filtering, so we must stub it.
  def take_at(schedule_id:, taken_at:, schedule: nil)
    take = instance_double(MedicationTake, schedule_id: schedule_id, taken_at: taken_at)
    allow(take).to receive(:schedule).and_return(schedule)
    take
  end

  # Build a context double that satisfies all methods the detector calls.
  def context_with(schedules:, takes:, start_date:, end_date:)
    instance_double(
      SmartInsights::Context,
      schedules: schedules,
      takes: takes,
      start_date: start_date,
      end_date: end_date
    )
  end

  # Resolve what `expected_time` would produce for a given date and "HH:MM" string.
  def expected_time(date, time_str)
    hour, minute = time_str.split(':').map(&:to_i)
    date.in_time_zone.change(hour: hour, min: minute)
  end

  let(:start_date) { Date.new(2024, 1, 1) }
  let(:single_day_end) { start_date } # 1 day = 1 occurrence
  let(:time_str) { '08:00' }

  context 'when there are no schedules (zero occurrences)' do
    it 'returns no insights' do
      ctx = context_with(schedules: [], takes: [], start_date: start_date, end_date: start_date)
      expect(described_class.new(ctx).call).to eq([])
    end
  end

  context 'when there is not enough timing evidence (< MINIMUM_EVENTS occurrences)' do
    it 'returns no insights for 1 occurrence (below threshold of 5)' do
      # MINIMUM_EVENTS = 5; use 1-day range with 1 schedule → 1 occurrence — not enough
      sched = schedule_double(id: 1, time: time_str, start_date: start_date, end_date: single_day_end)
      ctx = context_with(schedules: [sched], takes: [], start_date: start_date, end_date: single_day_end)
      expect(described_class.new(ctx).call).to eq([])
    end
  end

  context 'with enough evidence' do
    # 5 days × 1 schedule × 1 time = 5 occurrences (meets MINIMUM_EVENTS = 5)
    let(:end_date) { start_date + 4.days }
    let(:sched) { schedule_double(id: 42, time: time_str, start_date: start_date, end_date: end_date) }

    def takes_for_all_days(on_time: true)
      (start_date..end_date).map do |date|
        base = expected_time(date, time_str)
        offset = on_time ? 0.minutes : (described_class::WINDOW_MINUTES + 1).minutes
        take_at(schedule_id: 42, taken_at: base + offset, schedule: sched)
      end
    end

    it 'returns no insights when on-time ratio is below 0.8' do
      # Only 1 of 5 takes is on-time → ratio = 0.2
      takes = (start_date..end_date).map.with_index do |date, i|
        base = expected_time(date, time_str)
        offset = i.zero? ? 0.minutes : (described_class::WINDOW_MINUTES + 1).minutes
        take_at(schedule_id: 42, taken_at: base + offset, schedule: sched)
      end
      ctx = context_with(schedules: [sched], takes: takes, start_date: start_date, end_date: end_date)
      expect(described_class.new(ctx).call).to eq([])
    end

    it 'returns no insights when 4 of 5 takes are on-time (ratio 0.8 is the threshold boundary, not below it)' do
      # 3 of 5 on-time → ratio = 0.6 → silent
      takes = (start_date..end_date).map.with_index do |date, i|
        base = expected_time(date, time_str)
        offset = i < 3 ? 0.minutes : (described_class::WINDOW_MINUTES + 1).minutes
        take_at(schedule_id: 42, taken_at: base + offset, schedule: sched)
      end
      ctx = context_with(schedules: [sched], takes: takes, start_date: start_date, end_date: end_date)
      expect(described_class.new(ctx).call).to eq([])
    end

    it 'emits an insight when exactly 4 of 5 takes are on-time (ratio = 0.8) with correct percentage in summary' do
      # 4 of 5 on-time → ratio = 0.8 → emits insight, summary shows 80%
      takes = (start_date..end_date).map.with_index do |date, i|
        base = expected_time(date, time_str)
        offset = i < 4 ? 0.minutes : (described_class::WINDOW_MINUTES + 1).minutes
        take_at(schedule_id: 42, taken_at: base + offset, schedule: sched)
      end
      ctx = context_with(schedules: [sched], takes: takes, start_date: start_date, end_date: end_date)
      insights = described_class.new(ctx).call
      insight = insights.first
      expect(insights.size).to eq(1)
      summary_key = 'smart_insights.detectors.timing_consistency.summary'
      metric_key  = 'smart_insights.detectors.timing_consistency.metric_value'
      expect(insight.summary).to eq(I18n.t(summary_key, percentage: 80))
      expect(insight.metric_value).to eq(I18n.t(metric_key, count: 4))
    end

    it 'emits a positive timing_consistency insight (key/family/severity) when all takes are on-time' do
      # All 5 takes are exactly on time → ratio = 1.0
      ctx = context_with(
        schedules: [sched],
        takes: takes_for_all_days(on_time: true),
        start_date: start_date,
        end_date: end_date
      )
      insight = described_class.new(ctx).call.first
      expect(insight).to have_attributes(key: :timing_consistency, family: :timing, severity: :positive)
    end

    it 'sets correct I18n fields on the timing_consistency insight' do
      ctx = context_with(
        schedules: [sched],
        takes: takes_for_all_days(on_time: true),
        start_date: start_date,
        end_date: end_date
      )
      insight = described_class.new(ctx).call.first
      summary_key  = 'smart_insights.detectors.timing_consistency.summary'
      metric_key   = 'smart_insights.detectors.timing_consistency.metric_value'
      expect(insight.title).to eq(I18n.t('smart_insights.detectors.timing_consistency.title'))
      expect(insight.summary).to eq(I18n.t(summary_key, percentage: 100))
      expect(insight.detail).to eq(I18n.t('smart_insights.detectors.timing_consistency.detail'))
      expect(insight.metric_label).to eq(I18n.t('smart_insights.detectors.timing_consistency.metric_label'))
      expect(insight.metric_value).to eq(I18n.t(metric_key, count: 5))
    end

    it 'does not count a take outside the 90-minute described_class::WINDOW_MINUTES as on-time' do
      # All takes are 91 minutes outside described_class::WINDOW_MINUTES → ratio = 0.0
      ctx = context_with(
        schedules: [sched],
        takes: takes_for_all_days(on_time: false),
        start_date: start_date,
        end_date: end_date
      )
      expect(described_class.new(ctx).call).to eq([])
    end

    it 'counts a take exactly at the described_class::WINDOW_MINUTES boundary (90 min) as on-time' do
      # Takes land exactly 90 minutes from expected → still on-time (abs <= 90 min)
      takes = (start_date..end_date).map do |date|
        take_at(schedule_id: 42, taken_at: expected_time(date, time_str) + described_class::WINDOW_MINUTES.minutes, schedule: sched)
      end
      ctx = context_with(schedules: [sched], takes: takes, start_date: start_date, end_date: end_date)
      insights = described_class.new(ctx).call
      expect(insights.size).to eq(1)
    end

    it 'does not count a take for a different schedule as matching the occurrence' do
      # Takes have schedule_id: 99 (different), sched expects id: 42 → no match → ratio = 0
      other_sched = instance_double(Schedule, id: 99, schedule_config: { 'times' => [time_str] })
      (start_date..end_date).each do |date|
        allow(other_sched).to receive(:expected_doses_on).with(date).and_return(1)
      end
      takes = (start_date..end_date).map do |date|
        take_at(schedule_id: 99, taken_at: expected_time(date, time_str), schedule: sched)
      end
      ctx = context_with(schedules: [sched], takes: takes, start_date: start_date, end_date: end_date)
      expect(described_class.new(ctx).call).to eq([])
    end

    it 'ignores takes for schedules with no configured times' do
      # Schedule has no times → no occurrences generated → not enough evidence
      no_time_sched = instance_double(Schedule, id: 1, schedule_config: { 'times' => [] })
      (start_date..end_date).each do |date|
        allow(no_time_sched).to receive(:expected_doses_on).with(date).and_return(1)
      end
      # takes whose schedule has no configured times are filtered out of takes_with_configured_times
      no_time_schedule_take = take_at(schedule_id: 1, taken_at: start_date.in_time_zone, schedule: no_time_sched)
      ctx = context_with(schedules: [no_time_sched], takes: [no_time_schedule_take],
                         start_date: start_date, end_date: end_date)
      expect(described_class.new(ctx).call).to eq([])
    end

    it 'emits insights when occurrences exceed the minimum (> 5), not just equal to it' do
      # 6 days × 1 schedule × 1 time = 6 occurrences — tests >= vs == boundary
      extended_end = start_date + 5.days
      ext_sched = schedule_double(id: 42, time: time_str, start_date: start_date, end_date: extended_end)
      takes = (start_date..extended_end).map do |date|
        take_at(schedule_id: 42, taken_at: expected_time(date, time_str), schedule: ext_sched)
      end
      ctx = context_with(schedules: [ext_sched], takes: takes, start_date: start_date, end_date: extended_end)
      expect(described_class.new(ctx).call.size).to eq(1)
    end

    it 'skips days where expected_doses_on returns 0 when counting occurrences' do
      # Build a schedule where day 1 has 0 expected doses; only 4 other days have doses → not enough evidence
      sched_with_zero = instance_double(Schedule, id: 42, schedule_config: { 'times' => [time_str] })
      allow(sched_with_zero).to receive(:expected_doses_on).with(start_date).and_return(0)
      (1..4).each do |i|
        allow(sched_with_zero).to receive(:expected_doses_on).with(start_date + i.days).and_return(1)
      end
      ctx = context_with(schedules: [sched_with_zero], takes: [], start_date: start_date, end_date: end_date)
      # 4 occurrences < MINIMUM_EVENTS (5) → not enough evidence → []
      expect(described_class.new(ctx).call).to eq([])
    end

    it 'ignores takes for schedules with blank-string times' do
      # Schedule has ['', nil] → compact_blank removes them → no times → no occurrences
      blank_time_sched = instance_double(Schedule, id: 1, schedule_config: { 'times' => ['', nil] })
      (start_date..end_date).each do |date|
        allow(blank_time_sched).to receive(:expected_doses_on).with(date).and_return(1)
      end
      take_for_blank = take_at(schedule_id: 1, taken_at: start_date.in_time_zone, schedule: blank_time_sched)
      ctx = context_with(schedules: [blank_time_sched], takes: [take_for_blank],
                         start_date: start_date, end_date: end_date)
      expect(described_class.new(ctx).call).to eq([])
    end
  end
end
