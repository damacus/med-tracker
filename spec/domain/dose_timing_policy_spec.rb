# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DoseTimingPolicy do
  let(:now) { Time.current }

  def policy(takes: [], max_daily_doses: nil, min_hours_between_doses: nil, dose_cycle: 'daily')
    described_class.new(
      takes: takes,
      max_daily_doses: max_daily_doses,
      min_hours_between_doses: min_hours_between_doses,
      dose_cycle: dose_cycle
    )
  end

  def fake_take(taken_at:)
    instance_double('MedicationTake', taken_at: taken_at)
  end

  describe '#has_restrictions?' do
    it 'returns false when no restrictions set' do
      expect(policy.has_restrictions?).to be false
    end

    it 'returns true when max_daily_doses set' do
      expect(policy(max_daily_doses: 3).has_restrictions?).to be true
    end

    it 'returns true when min_hours_between_doses set' do
      expect(policy(min_hours_between_doses: 4).has_restrictions?).to be true
    end
  end

  describe '#can_take_at?' do
    context 'without restrictions' do
      it 'returns true' do
        expect(policy.can_take_at?).to be true
      end
    end

    context 'with max_daily_doses' do
      it 'returns false when limit reached' do
        take = fake_take(taken_at: now.beginning_of_day + 8.hours)
        expect(policy(max_daily_doses: 1, takes: [take]).can_take_at?).to be false
      end

      it 'returns true when under limit' do
        expect(policy(max_daily_doses: 2).can_take_at?).to be true
      end
    end

    context 'with min_hours_between_doses' do
      it 'returns false when minimum hours not passed' do
        take = fake_take(taken_at: 2.hours.ago)
        expect(policy(min_hours_between_doses: 4, takes: [take]).can_take_at?).to be false
      end

      it 'returns true when minimum hours have passed' do
        take = fake_take(taken_at: 5.hours.ago)
        expect(policy(min_hours_between_doses: 4, takes: [take]).can_take_at?).to be true
      end
    end
  end

  describe '#next_available_time' do
    it 'returns nil without restrictions' do
      expect(policy.next_available_time).to be_nil
    end

    it 'returns approximately now when can take immediately' do
      expect(policy(max_daily_doses: 3).next_available_time).to be_within(1.second).of(now)
    end

    it 'calculates time based on min_hours_between_doses' do
      take = fake_take(taken_at: 2.hours.ago)
      expected = take.taken_at + 4.hours
      result = policy(min_hours_between_doses: 4, takes: [take]).next_available_time
      expect(result).to be_within(1.second).of(expected)
    end

    it 'calculates end of cycle + 1 second when max doses reached (daily)' do
      take = fake_take(taken_at: now.beginning_of_day + 8.hours)
      result = policy(max_daily_doses: 1, takes: [take]).next_available_time
      expect(result).to be_within(1.second).of(now.end_of_day + 1.second)
    end

    it 'calculates end of week + 1 second for weekly cycle' do
      take = fake_take(taken_at: now.beginning_of_week + 1.hour)
      result = policy(max_daily_doses: 1, dose_cycle: 'weekly', takes: [take]).next_available_time
      expect(result).to be_within(1.second).of(now.end_of_week + 1.second)
    end

    it 'returns earliest satisfying time when both restrictions apply' do
      take = fake_take(taken_at: 2.hours.ago)
      result = policy(max_daily_doses: 3, min_hours_between_doses: 4, takes: [take]).next_available_time
      expect(result).to be_within(1.second).of(take.taken_at + 4.hours)
    end
  end

  describe '#time_until_next_dose' do
    it 'returns nil when can take now' do
      expect(policy.time_until_next_dose).to be_nil
    end

    it 'returns seconds until next available time' do
      take = fake_take(taken_at: 2.hours.ago)
      p = policy(min_hours_between_doses: 4, takes: [take])
      expected = (p.next_available_time - now).to_i
      expect(p.time_until_next_dose).to be_within(2).of(expected)
    end
  end

  describe '#countdown_display' do
    it 'returns nil when can take now' do
      expect(policy.countdown_display).to be_nil
    end

    it 'formats hours and minutes' do
      take = fake_take(taken_at: 2.hours.ago)
      expect(policy(min_hours_between_doses: 4, takes: [take]).countdown_display).to match(/\d+h \d+m/)
    end

    it 'formats minutes only' do
      take = fake_take(taken_at: (4.hours - 30.minutes).ago)
      expect(policy(min_hours_between_doses: 4, takes: [take]).countdown_display).to match(/^\d+m$/)
    end

    it 'returns "less than 1 minute" for < 60 seconds' do
      take = fake_take(taken_at: (4.hours - 30.seconds).ago)
      expect(policy(min_hours_between_doses: 4, takes: [take]).countdown_display).to eq('less than 1 minute')
    end
  end
end
