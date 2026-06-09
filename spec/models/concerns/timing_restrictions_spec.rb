# frozen_string_literal: true

require 'rails_helper'

# Tested through Schedule, which includes TimingRestrictions.
RSpec.describe TimingRestrictions do
  # Build a schedule with explicit timing constraints so tests are deterministic
  let(:schedule) do
    create(:schedule,
           max_daily_doses: 3,
           min_hours_between_doses: 4,
           start_date: 30.days.ago.to_date,
           end_date: 1.year.from_now.to_date)
  end

  describe '#dose_constraints' do
    it 'returns a DoseConstraints with the schedule values' do
      dc = schedule.dose_constraints
      expect(dc).to be_a(DoseConstraints)
      expect(dc.max_daily_doses).to eq(3)
      expect(dc.min_hours_between_doses).to eq(4)
    end

    it 'is memoized' do
      first  = schedule.dose_constraints
      second = schedule.dose_constraints
      expect(first).to be(second)
    end
  end

  describe '#timing_policy' do
    it 'returns a DoseTimingPolicy' do
      expect(schedule.timing_policy).to be_a(DoseTimingPolicy)
    end

    it 'is memoized' do
      first  = schedule.timing_policy
      second = schedule.timing_policy
      expect(first).to be(second)
    end
  end

  describe '#timing_restrictions? / restrictions?' do
    it 'returns true when max_daily_doses or min_hours_between_doses are set' do
      expect(schedule.timing_restrictions?).to be(true)
    end

    it 'returns false when neither constraint is set' do
      unconstrained = create(:schedule,
                             max_daily_doses: nil,
                             min_hours_between_doses: nil)
      expect(unconstrained.timing_restrictions?).to be(false)
    end
  end

  describe '#can_take_at?' do
    it 'returns true immediately for an unconstrained schedule' do
      unconstrained = create(:schedule, max_daily_doses: nil, min_hours_between_doses: nil)
      expect(unconstrained.can_take_at?).to be(true)
    end

    it 'returns true for a fresh constrained schedule with no takes yet' do
      # No takes recorded → no cooldown, no daily limit hit
      expect(schedule.can_take_at?).to be(true)
    end
  end

  describe '#can_take_now?' do
    it 'delegates to can_take_at? with the current time' do
      expect(schedule.can_take_now?).to eq(schedule.can_take_at?)
    end
  end

  describe '#can_administer?' do
    it 'returns false when the medication is out of stock' do
      allow(schedule.medication).to receive(:out_of_stock?).and_return(true)
      expect(schedule.can_administer?).to be(false)
    end

    it 'returns true when in-stock and not in cooldown' do
      allow(schedule.medication).to receive(:out_of_stock?).and_return(false)
      expect(schedule.can_administer?).to be(true)
    end
  end

  describe '#administration_blocked_reason' do
    it 'returns :out_of_stock when the medication is out of stock' do
      allow(schedule.medication).to receive(:out_of_stock?).and_return(true)
      expect(schedule.administration_blocked_reason).to eq(:out_of_stock)
    end

    it 'returns nil when both stock and timing are OK' do
      allow(schedule.medication).to receive(:out_of_stock?).and_return(false)
      expect(schedule.administration_blocked_reason).to be_nil
    end
  end

  describe '#reload clears memoized state' do
    it 'clears dose_constraints and timing_policy on reload' do
      original_constraints = schedule.dose_constraints
      original_policy      = schedule.timing_policy

      schedule.reload

      # After reload the memoized objects are new instances
      expect(schedule.dose_constraints).not_to be(original_constraints)
      expect(schedule.timing_policy).not_to be(original_policy)
    end
  end

  describe 'delegated timing-policy methods' do
    it 'responds to next_available_time' do
      expect(schedule).to respond_to(:next_available_time)
    end

    it 'responds to time_until_next_dose' do
      expect(schedule).to respond_to(:time_until_next_dose)
    end

    it 'responds to countdown_display' do
      expect(schedule).to respond_to(:countdown_display)
    end
  end
end
