# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DoseConstraints do
  let(:now) { Time.current }
  let(:daily_cycle) { DoseCycle.new('daily') }

  def constraints(max_daily_doses: nil, min_hours_between_doses: nil)
    described_class.new(
      max_daily_doses: max_daily_doses,
      min_hours_between_doses: min_hours_between_doses
    )
  end

  def fake_take(taken_at:)
    instance_double(MedicationTake, taken_at: taken_at)
  end

  describe '#restrictions?' do
    it 'returns false when no restrictions are set' do
      expect(constraints.restrictions?).to be false
    end

    it 'returns false when values are blank' do
      expect(described_class.new(max_daily_doses: '', min_hours_between_doses: nil).restrictions?).to be false
    end

    it 'returns true when the daily limit is present' do
      expect(constraints(max_daily_doses: 2).restrictions?).to be true
    end

    it 'returns true when the interval limit is present' do
      expect(constraints(min_hours_between_doses: 4).restrictions?).to be true
    end
  end

  describe '#daily_limit?' do
    it 'returns true when max_daily_doses is present' do
      expect(constraints(max_daily_doses: 2).daily_limit?).to be true
    end

    it 'returns false when max_daily_doses is blank' do
      expect(described_class.new(max_daily_doses: '', min_hours_between_doses: nil).daily_limit?).to be false
    end
  end

  describe '#interval_limit?' do
    it 'returns true when min_hours_between_doses is present' do
      expect(constraints(min_hours_between_doses: 4).interval_limit?).to be true
    end

    it 'returns false when min_hours_between_doses is blank' do
      expect(described_class.new(max_daily_doses: nil, min_hours_between_doses: '').interval_limit?).to be false
    end
  end

  describe '#would_exceed_daily_limit?' do
    it 'returns false when there is no daily limit' do
      expect(constraints.would_exceed_daily_limit?(takes: [], cycle: daily_cycle)).to be false
    end

    it 'returns true when doses in cycle meet the daily limit' do
      take = fake_take(taken_at: now.beginning_of_day + 2.hours)

      expect(constraints(max_daily_doses: 1).would_exceed_daily_limit?(takes: [take], cycle: daily_cycle)).to be true
    end

    it 'ignores doses outside the current cycle' do
      yesterday_take = fake_take(taken_at: 1.day.ago.beginning_of_day + 2.hours)

      expect(
        constraints(max_daily_doses: 1).would_exceed_daily_limit?(takes: [yesterday_take], cycle: daily_cycle)
      ).to be false
    end
  end

  describe '#would_violate_interval?' do
    it 'returns false when there is no interval limit' do
      expect(constraints.would_violate_interval?(takes: [], check_time: now)).to be false
    end

    it 'returns true when the latest prior take is inside the interval window' do
      take = fake_take(taken_at: 2.hours.ago)

      expect(constraints(min_hours_between_doses: 4).would_violate_interval?(takes: [take], check_time: now)).to be true
    end

    it 'returns false when the latest prior take is outside the interval window' do
      take = fake_take(taken_at: 5.hours.ago)

      expect(
        constraints(min_hours_between_doses: 4).would_violate_interval?(takes: [take], check_time: now)
      ).to be false
    end

    it 'ignores takes after the check time' do
      take = fake_take(taken_at: 30.minutes.from_now)

      expect(
        constraints(min_hours_between_doses: 4).would_violate_interval?(takes: [take], check_time: now)
      ).to be false
    end
  end

  describe '#satisfied_by?' do
    it 'returns true when there are no constraints' do
      expect(constraints.satisfied_by?(takes: [], check_time: now, cycle: daily_cycle)).to be true
    end

    it 'returns false when the daily limit would be exceeded' do
      take = fake_take(taken_at: now.beginning_of_day + 1.hour)

      expect(
        constraints(max_daily_doses: 1).satisfied_by?(takes: [take], check_time: now, cycle: daily_cycle)
      ).to be false
    end

    it 'returns false when the interval limit would be violated' do
      take = fake_take(taken_at: 2.hours.ago)

      expect(
        constraints(min_hours_between_doses: 4).satisfied_by?(takes: [take], check_time: now, cycle: daily_cycle)
      ).to be false
    end

    it 'returns true when both constraints are satisfied' do
      take = fake_take(taken_at: 5.hours.ago)

      expect(
        constraints(max_daily_doses: 2, min_hours_between_doses: 4).satisfied_by?(
          takes: [take],
          check_time: now,
          cycle: daily_cycle
        )
      ).to be true
    end
  end

  describe '#next_available_time' do
    it 'returns nil when there are no constraints' do
      expect(constraints.next_available_time(takes: [], cycle: daily_cycle, now: now)).to be_nil
    end

    it 'returns now when constrained but currently satisfied' do
      expect(
        constraints(max_daily_doses: 3).next_available_time(takes: [], cycle: daily_cycle, now: now)
      ).to be_within(1.second).of(now)
    end

    it 'returns the next cycle reset when blocked by the daily limit' do
      take = fake_take(taken_at: now.beginning_of_day + 8.hours)

      expect(
        constraints(max_daily_doses: 1).next_available_time(takes: [take], cycle: daily_cycle, now: now)
      ).to be_within(1.second).of(now.end_of_day + 1.second)
    end

    it 'returns the interval expiry when blocked by the interval limit' do
      take = fake_take(taken_at: 2.hours.ago)
      expected = take.taken_at + 4.hours

      expect(
        constraints(min_hours_between_doses: 4).next_available_time(takes: [take], cycle: daily_cycle, now: now)
      ).to be_within(1.second).of(expected)
    end

    it 'returns the later blocker when both constraints apply' do
      take = fake_take(taken_at: now.beginning_of_day + 22.hours)
      expected = take.taken_at + 4.hours

      expect(
        constraints(max_daily_doses: 1, min_hours_between_doses: 4).next_available_time(
          takes: [take],
          cycle: daily_cycle,
          now: now
        )
      ).to be_within(1.second).of(expected)
    end
  end
end
