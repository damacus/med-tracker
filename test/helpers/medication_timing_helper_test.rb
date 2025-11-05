# frozen_string_literal: true

require 'test_helper'

class MedicationTimingHelperTest < ActionView::TestCase
  test 'time_until_available returns nil when next_time is nil' do
    assert_nil time_until_available(nil)
  end

  test 'time_until_available returns "Available now" when next_time is in the past' do
    next_time = 5.minutes.ago
    assert_equal 'Available now', time_until_available(next_time)
  end

  test 'time_until_available returns "Available now" when next_time is now' do
    next_time = Time.current
    assert_equal 'Available now', time_until_available(next_time)
  end

  test 'time_until_available returns minutes only when less than an hour away' do
    next_time = 30.minutes.from_now
    assert_equal 'Available in 30m', time_until_available(next_time)
  end

  test 'time_until_available rounds down partial minutes' do
    next_time = 30.minutes.from_now + 30.seconds
    assert_equal 'Available in 30m', time_until_available(next_time)
  end

  test 'time_until_available returns hours only when exact hours' do
    next_time = 2.hours.from_now
    assert_equal 'Available in 2h', time_until_available(next_time)
  end

  test 'time_until_available returns hours and minutes when both present' do
    next_time = 2.hours.from_now + 30.minutes
    assert_equal 'Available in 2h 30m', time_until_available(next_time)
  end

  test 'time_until_available handles single hour and minutes' do
    next_time = 1.hour.from_now + 15.minutes
    assert_equal 'Available in 1h 15m', time_until_available(next_time)
  end

  test 'time_until_available rounds down partial minutes with hours' do
    next_time = 3.hours.from_now + 45.minutes + 45.seconds
    assert_equal 'Available in 3h 45m', time_until_available(next_time)
  end

  test 'time_until_available formats correctly for long durations' do
    next_time = 23.hours.from_now + 59.minutes
    assert_equal 'Available in 23h 59m', time_until_available(next_time)
  end
end
