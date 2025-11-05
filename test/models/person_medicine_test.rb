# frozen_string_literal: true

require 'test_helper'

class PersonMedicineTest < ActiveSupport::TestCase
  setup do
    @person_medicine = person_medicines(:john_vitamin_d)
  end

  test 'timing_restrictions? returns true when max_daily_doses is present' do
    @person_medicine.update(max_daily_doses: 4, min_hours_between_doses: nil)
    assert @person_medicine.timing_restrictions?
  end

  test 'timing_restrictions? returns true when min_hours_between_doses is present' do
    @person_medicine.update(max_daily_doses: nil, min_hours_between_doses: 6)
    assert @person_medicine.timing_restrictions?
  end

  test 'timing_restrictions? returns false when no restrictions are present' do
    @person_medicine.update(max_daily_doses: nil, min_hours_between_doses: nil)
    assert_not @person_medicine.timing_restrictions?
  end

  test 'cycle_period returns 1 day' do
    assert_equal 1.day, @person_medicine.cycle_period
  end

  test 'can_take_now? returns true when no timing restrictions' do
    @person_medicine.update(max_daily_doses: nil, min_hours_between_doses: nil)
    assert @person_medicine.can_take_now?
  end

  test 'can_take_now? returns true when under max doses for today' do
    @person_medicine.update(max_daily_doses: 2, min_hours_between_doses: nil)
    @person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
    assert @person_medicine.can_take_now?
  end

  test 'can_take_now? returns false when max doses reached for today' do
    @person_medicine.update(max_daily_doses: 2, min_hours_between_doses: nil)
    @person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
    @person_medicine.medication_takes.create!(taken_at: 4.hours.ago)
    assert_not @person_medicine.can_take_now?
  end

  test 'can_take_now? returns true the next day after max doses' do
    @person_medicine.update(max_daily_doses: 2, min_hours_between_doses: nil)
    @person_medicine.medication_takes.create!(taken_at: 1.day.ago.beginning_of_day + 8.hours)
    @person_medicine.medication_takes.create!(taken_at: 1.day.ago.beginning_of_day + 12.hours)
    assert @person_medicine.can_take_now?
  end

  test 'can_take_now? returns true when no previous doses with min hours restriction' do
    @person_medicine.update(max_daily_doses: nil, min_hours_between_doses: 4)
    assert @person_medicine.can_take_now?
  end

  test 'can_take_now? returns true when enough time has passed' do
    @person_medicine.update(max_daily_doses: nil, min_hours_between_doses: 4)
    @person_medicine.medication_takes.create!(taken_at: 5.hours.ago)
    assert @person_medicine.can_take_now?
  end

  test 'can_take_now? returns false when not enough time has passed' do
    @person_medicine.update(max_daily_doses: nil, min_hours_between_doses: 4)
    @person_medicine.medication_takes.create!(taken_at: 2.hours.ago)
    assert_not @person_medicine.can_take_now?
  end

  test 'can_take_now? returns false when max doses reached even if hours passed' do
    @person_medicine.update(max_daily_doses: 3, min_hours_between_doses: 4)
    @person_medicine.medication_takes.create!(taken_at: 12.hours.ago)
    @person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
    @person_medicine.medication_takes.create!(taken_at: 5.hours.ago)
    assert_not @person_medicine.can_take_now?
  end

  test 'can_take_now? returns false when not enough hours passed even if under max doses' do
    @person_medicine.update(max_daily_doses: 3, min_hours_between_doses: 4)
    @person_medicine.medication_takes.create!(taken_at: 2.hours.ago)
    assert_not @person_medicine.can_take_now?
  end

  test 'next_available_time returns nil when no timing restrictions' do
    @person_medicine.update(max_daily_doses: nil, min_hours_between_doses: nil)
    assert_nil @person_medicine.next_available_time
  end

  test 'next_available_time returns current time when can take now' do
    @person_medicine.update(max_daily_doses: 4, min_hours_between_doses: 4)
    assert_in_delta Time.current.to_i, @person_medicine.next_available_time.to_i, 1
  end

  test 'next_available_time returns time when min hours will be satisfied' do
    @person_medicine.update(max_daily_doses: nil, min_hours_between_doses: 4)
    @person_medicine.medication_takes.create!(taken_at: 2.hours.ago)
    expected_time = 2.hours.from_now
    assert_in_delta expected_time.to_i, @person_medicine.next_available_time.to_i, 60
  end

  test 'next_available_time returns start of next day when max doses reached' do
    @person_medicine.update(max_daily_doses: 2, min_hours_between_doses: nil)
    @person_medicine.medication_takes.create!(taken_at: 8.hours.ago)
    @person_medicine.medication_takes.create!(taken_at: 4.hours.ago)
    expected_time = Time.current.end_of_day + 1.second
    assert_in_delta expected_time.to_i, @person_medicine.next_available_time.to_i, 60
  end
end
