# frozen_string_literal: true

require 'test_helper'

class PrescriptionTest < ActiveSupport::TestCase
  setup do
    @prescription = prescriptions(:john_paracetamol)
  end

  test 'timing_restrictions? returns true when max_daily_doses is present' do
    @prescription.update(max_daily_doses: 4, min_hours_between_doses: nil)
    assert @prescription.timing_restrictions?
  end

  test 'timing_restrictions? returns true when min_hours_between_doses is present' do
    @prescription.update(max_daily_doses: nil, min_hours_between_doses: 6)
    assert @prescription.timing_restrictions?
  end

  test 'timing_restrictions? returns false when no restrictions are present' do
    @prescription.update(max_daily_doses: nil, min_hours_between_doses: nil)
    assert_not @prescription.timing_restrictions?
  end

  test 'can_take_now? returns true when no timing restrictions' do
    @prescription.update(max_daily_doses: nil, min_hours_between_doses: nil)
    assert @prescription.can_take_now?
  end

  test 'can_take_now? returns true when under max doses for today' do
    @prescription.update(max_daily_doses: 2, min_hours_between_doses: nil)
    @prescription.medication_takes.create!(taken_at: 8.hours.ago)
    assert @prescription.can_take_now?
  end

  test 'can_take_now? returns false when max doses reached for today' do
    @prescription.update(max_daily_doses: 2, min_hours_between_doses: nil)
    @prescription.medication_takes.create!(taken_at: 8.hours.ago)
    @prescription.medication_takes.create!(taken_at: 4.hours.ago)
    assert_not @prescription.can_take_now?
  end

  test 'can_take_now? returns true the next day after max doses' do
    @prescription.update(max_daily_doses: 2, min_hours_between_doses: nil)
    @prescription.medication_takes.create!(taken_at: 1.day.ago.beginning_of_day + 8.hours)
    @prescription.medication_takes.create!(taken_at: 1.day.ago.beginning_of_day + 12.hours)
    assert @prescription.can_take_now?
  end

  test 'can_take_now? returns true when no previous doses with min hours restriction' do
    @prescription.update(max_daily_doses: nil, min_hours_between_doses: 4)
    assert @prescription.can_take_now?
  end

  test 'can_take_now? returns true when enough time has passed' do
    @prescription.update(max_daily_doses: nil, min_hours_between_doses: 4)
    @prescription.medication_takes.create!(taken_at: 5.hours.ago)
    assert @prescription.can_take_now?
  end

  test 'can_take_now? returns false when not enough time has passed' do
    @prescription.update(max_daily_doses: nil, min_hours_between_doses: 4)
    @prescription.medication_takes.create!(taken_at: 2.hours.ago)
    assert_not @prescription.can_take_now?
  end

  test 'can_take_now? returns false when max doses reached even if hours passed' do
    @prescription.update(max_daily_doses: 3, min_hours_between_doses: 4)
    @prescription.medication_takes.create!(taken_at: 12.hours.ago)
    @prescription.medication_takes.create!(taken_at: 8.hours.ago)
    @prescription.medication_takes.create!(taken_at: 5.hours.ago)
    assert_not @prescription.can_take_now?
  end

  test 'can_take_now? returns false when not enough hours passed even if under max doses' do
    @prescription.update(max_daily_doses: 3, min_hours_between_doses: 4)
    @prescription.medication_takes.create!(taken_at: 2.hours.ago)
    assert_not @prescription.can_take_now?
  end

  test 'next_available_time returns nil when no timing restrictions' do
    @prescription.update(max_daily_doses: nil, min_hours_between_doses: nil)
    assert_nil @prescription.next_available_time
  end

  test 'next_available_time returns current time when can take now' do
    @prescription.update(max_daily_doses: 4, min_hours_between_doses: 4)
    assert_in_delta Time.current.to_i, @prescription.next_available_time.to_i, 1
  end

  test 'next_available_time returns time when min hours will be satisfied' do
    @prescription.update(max_daily_doses: nil, min_hours_between_doses: 4)
    @prescription.medication_takes.create!(taken_at: 2.hours.ago)
    expected_time = 2.hours.from_now
    assert_in_delta expected_time.to_i, @prescription.next_available_time.to_i, 60
  end

  test 'next_available_time returns start of next day when max doses reached' do
    @prescription.update(max_daily_doses: 2, min_hours_between_doses: nil)
    @prescription.medication_takes.create!(taken_at: 8.hours.ago)
    @prescription.medication_takes.create!(taken_at: 4.hours.ago)
    expected_time = Time.current.end_of_day + 1.second
    assert_in_delta expected_time.to_i, @prescription.next_available_time.to_i, 60
  end
end
