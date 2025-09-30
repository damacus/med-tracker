# frozen_string_literal: true

require 'test_helper'

class TakeMedicineTest < ActiveSupport::TestCase
  setup do
    @person = people(:adult_john)
    @medicine = medicines(:ibuprofen)

    # Create a prescription with timing restrictions
    @prescription = Prescription.create!(
      person: @person,
      medicine: @medicine,
      dosage: '200mg',
      frequency: 'As needed',
      start_date: Date.current - 1.month,
      max_daily_doses: 3,
      min_hours_between_doses: 6,
      dose_cycle: 'daily'
    )

    # Reference time for testing
    @reference_time = Time.current.change(hour: 12, min: 0) # Noon today
  end

  test 'validates presence of taken_at' do
    take_medicine = TakeMedicine.new(
      prescription: @prescription,
      amount_ml: 5
    )
    assert_not take_medicine.valid?
    assert_includes take_medicine.errors[:taken_at], "can't be blank"
  end

  test 'validates presence of amount_ml' do
    take_medicine = TakeMedicine.new(
      prescription: @prescription,
      taken_at: Time.current
    )
    assert_not take_medicine.valid?
    assert_includes take_medicine.errors[:amount_ml], "can't be blank"
  end

  test 'validates amount_ml is greater than 0' do
    take_medicine = TakeMedicine.new(
      prescription: @prescription,
      taken_at: Time.current,
      amount_ml: 0
    )
    assert_not take_medicine.valid?
    assert_includes take_medicine.errors[:amount_ml], 'must be greater than 0'
  end

  test 'validates taken_at is not in the future' do
    take_medicine = TakeMedicine.new(
      prescription: @prescription,
      taken_at: 1.hour.from_now,
      amount_ml: 5
    )
    assert_not take_medicine.valid?
    assert_includes take_medicine.errors[:taken_at], 'cannot be in the future'
  end

  test 'respects max_daily_doses restriction' do
    # Create a prescription with no min_hours restriction for this test
    prescription = Prescription.create!(
      person: @person,
      medicine: @medicine,
      dosage: '200mg',
      frequency: 'As needed',
      start_date: Date.current - 1.month,
      max_daily_doses: 3,
      dose_cycle: 'daily'
    )

    # Create 3 doses already taken today (max allowed)
    today = @reference_time.beginning_of_day

    # Morning dose at 8 AM
    TakeMedicine.new(prescription: prescription, taken_at: today + 8.hours, amount_ml: 5).save(validate: false)

    # Noon dose at 12 PM
    TakeMedicine.new(prescription: prescription, taken_at: today + 12.hours, amount_ml: 5).save(validate: false)

    # Evening dose at 6 PM
    TakeMedicine.new(prescription: prescription, taken_at: today + 18.hours, amount_ml: 5).save(validate: false)

    # Try to take a 4th dose at 9 PM
    take_medicine = TakeMedicine.new(
      prescription: prescription,
      taken_at: today + 21.hours,
      amount_ml: 5
    )

    assert_not take_medicine.valid?
    assert_includes take_medicine.errors[:base], 'Maximum of 3 doses per day allowed'
  end

  test 'respects min_hours_between_doses restriction' do
    # Create a dose taken 4 hours ago (less than min 6 hours)
    TakeMedicine.new(prescription: @prescription, taken_at: @reference_time - 4.hours,
                     amount_ml: 5).save(validate: false)

    # Try to take another dose now
    take_medicine = TakeMedicine.new(
      prescription: @prescription,
      taken_at: @reference_time,
      amount_ml: 5
    )

    assert_not take_medicine.valid?
    assert_includes take_medicine.errors[:base], 'Must wait at least 6 hours between doses'
  end

  test 'allows dose after minimum hours have passed' do
    # Create a dose taken 7 hours ago (more than min 6 hours)
    TakeMedicine.new(prescription: @prescription, taken_at: @reference_time - 7.hours,
                     amount_ml: 5).save(validate: false)

    travel_to @reference_time do
      # Try to take another dose now
      take_medicine = TakeMedicine.new(
        prescription: @prescription,
        taken_at: @reference_time,
        amount_ml: 5
      )

      assert take_medicine.valid?, 'Should be able to take dose after minimum hours have passed'
    end
  end

  test 'validates doses across day boundaries' do
    # Create a prescription with daily cycle
    prescription = Prescription.create!(
      person: @person,
      medicine: @medicine,
      dosage: '200mg',
      frequency: 'Daily',
      start_date: Date.current - 1.month,
      max_daily_doses: 2,
      min_hours_between_doses: 12,
      dose_cycle: 'daily'
    )

    # Create a dose taken at 11 PM yesterday
    TakeMedicine.new(
      prescription: prescription,
      taken_at: @reference_time.beginning_of_day - 1.hour, # 11 PM yesterday
      amount_ml: 5
    ).save(validate: false)

    # Try to take a dose at 8 AM today (9 hours later, less than 12 hour minimum)
    morning_dose = TakeMedicine.new(
      prescription: prescription,
      taken_at: @reference_time.beginning_of_day + 8.hours, # 8 AM today
      amount_ml: 5
    )

    assert_not morning_dose.valid?
    assert_includes morning_dose.errors[:base], 'Must wait at least 12 hours between doses'
  end

  test 'validates weekly cycle correctly' do
    # Create a prescription with weekly cycle
    weekly_prescription = Prescription.create!(
      person: @person,
      medicine: @medicine,
      dosage: '200mg',
      frequency: 'Weekly',
      start_date: Date.current - 1.month,
      max_daily_doses: 1,
      dose_cycle: 'weekly'
    )

    # Create a dose taken 3 days ago
    TakeMedicine.new(prescription: weekly_prescription, taken_at: @reference_time - 3.days,
                     amount_ml: 5).save(validate: false)

    # Try to take another dose within the same week
    take_medicine = TakeMedicine.new(
      prescription: weekly_prescription,
      taken_at: @reference_time,
      amount_ml: 5
    )

    assert_not take_medicine.valid?
    assert_includes take_medicine.errors[:base], 'Maximum of 1 doses per week allowed'
  end

  test 'ignores timing restrictions if none are set' do
    # Create a prescription without timing restrictions
    unrestricted_prescription = Prescription.create!(
      person: @person,
      medicine: @medicine,
      dosage: '200mg',
      frequency: 'As needed',
      start_date: Date.current - 1.month
    )

    # Create multiple doses in the same day
    3.times do |i|
      # Create historical doses by bypassing validations
      TakeMedicine.new(
        prescription: unrestricted_prescription,
        taken_at: @reference_time - (i * 2).hours, # 0, 2, 4 hours ago
        amount_ml: 5
      ).save(validate: false)
    end

    travel_to @reference_time do
      # Now, validate that a new dose can be taken
      take_medicine = TakeMedicine.new(
        prescription: unrestricted_prescription,
        taken_at: @reference_time,
        amount_ml: 5
      )

      assert take_medicine.valid?, 'Should allow unlimited doses when no restrictions'
    end
  end

  test 'calculates hours between times correctly' do
    take_medicine = TakeMedicine.new(prescription: @prescription)

    start_time = Time.current.change(hour: 9)
    end_time = Time.current.change(hour: 15)

    # Using the send method to test private method
    hours = take_medicine.send(:calculate_hours_between, start_time, end_time)
    assert_equal 6, hours
  end

  test 'finds last dose before given time' do
    # Create a prescription with no min_hours restriction for this test
    prescription = Prescription.create!(
      person: @person,
      medicine: @medicine,
      dosage: '200mg',
      frequency: 'As needed',
      start_date: Date.current - 1.month,
      max_daily_doses: 3,
      dose_cycle: 'daily'
    )

    # Create doses at different times
    dose1 = TakeMedicine.new(prescription: prescription, taken_at: @reference_time - 8.hours, amount_ml: 5)
    dose1.save(validate: false)

    dose2 = TakeMedicine.new(prescription: prescription, taken_at: @reference_time - 4.hours, amount_ml: 5)
    dose2.save(validate: false)

    take_medicine = TakeMedicine.new(prescription: prescription)

    # Using the send method to test private method
    last_dose = take_medicine.send(:find_last_dose_before, @reference_time)
    assert_equal dose2.id, last_dose.id
  end

  test 'handles prescription with no previous doses' do
    travel_to @reference_time do
      take_medicine = TakeMedicine.new(
        prescription: @prescription,
        taken_at: @reference_time,
        amount_ml: 5
      )

      assert take_medicine.valid?, 'Should be valid when there are no previous doses'
    end
  end
end
