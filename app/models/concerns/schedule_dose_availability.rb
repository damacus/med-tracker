# frozen_string_literal: true

module ScheduleDoseAvailability
  extend ActiveSupport::Concern

  def can_take_dose?(at: Time.current) = dose_blocked_reason(at: at).nil?

  def dose_blocked_reason(at: Time.current)
    at = normalize_time(at)
    return :inactive if at.blank? || !active_on?(at.to_date)
    return :stock if medication.out_of_stock?
    return :timing unless can_take_at?(at)

    nil
  end

  def next_dose_due_at(at: Time.current)
    at = normalize_time(at)
    return if at.blank?

    next_configured_due_at(at) || next_available_time
  end

  def overdue?(at: Time.current)
    at = normalize_time(at)
    return false if at.blank? || !active_on?(at.to_date) || schedule_type_prn?

    due_count = configured_due_times_for(at.to_date).count { |due_at| due_at <= at }
    due_count.positive? && taken_count_until(at) < due_count
  end

  private

  def normalize_time(value)
    return value.in_time_zone if value.respond_to?(:in_time_zone)

    Time.zone.parse(value.to_s)
  rescue ArgumentError, TypeError
    nil
  end

  def next_configured_due_at(at)
    return if configured_times.empty?

    due_at = configured_due_times_for(at.to_date).find { |configured_time| configured_time >= at }
    return due_at if due_at

    next_configured_due_on_future_date(at.to_date + 1.day)
  end

  def next_configured_due_on_future_date(date)
    return if date > end_date
    return configured_due_times_for(date).first if applies_on?(date)

    next_configured_due_on_future_date(date + 1.day)
  end

  def configured_due_times_for(date)
    return [] unless applies_on?(date)

    configured_times.filter_map { |time| configured_time_on(date, time) }.sort
  end

  def configured_time_on(date, value)
    hour, minute = value.to_s.split(':', 3).first(2)
    return unless hour&.match?(/\A\d+\z/) && minute&.match?(/\A\d+\z/)

    hour = hour.to_i
    minute = minute.to_i
    return unless (0..23).cover?(hour) && (0..59).cover?(minute)

    Time.zone.local(date.year, date.month, date.day, hour, minute)
  end

  def taken_count_until(at)
    range = at.beginning_of_day..at
    return medication_takes.count { |take| range.cover?(take.taken_at) } if medication_takes.loaded?

    medication_takes.where(taken_at: range).count
  end
end
