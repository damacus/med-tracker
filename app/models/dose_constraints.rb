# frozen_string_literal: true

# typed: true

require 'sorbet-runtime'
require 'bigdecimal'

class DoseConstraints
  extend T::Sig

  attr_reader :max_daily_doses, :min_hours_between_doses

  sig do
    params(
      max_daily_doses: T.nilable(T.any(Integer, String, BigDecimal)),
      min_hours_between_doses: T.nilable(T.any(Integer, String, BigDecimal))
    ).void
  end
  def initialize(max_daily_doses:, min_hours_between_doses:)
    @max_daily_doses = normalize(max_daily_doses)
    @min_hours_between_doses = normalize(min_hours_between_doses)
  end

  sig { returns(T::Boolean) }
  def restrictions?
    daily_limit? || interval_limit?
  end

  sig { returns(T::Boolean) }
  def daily_limit?
    max_daily_doses.present?
  end

  sig { returns(T::Boolean) }
  def interval_limit?
    min_hours_between_doses.present?
  end

  sig do
    params(
      takes: T::Array[MedicationTake],
      cycle: DoseCycle,
      check_time: Time
    ).returns(T::Boolean)
  end
  def would_exceed_daily_limit?(takes:, cycle:, check_time: Time.current)
    return false unless daily_limit?

    range = cycle.range_for(check_time)
    doses_in_cycle = takes.count { |take| range.cover?(take.taken_at) }

    doses_in_cycle >= T.cast(max_daily_doses, T.any(Integer, Float, Rational, BigDecimal))
  end

  sig { params(takes: T::Array[MedicationTake], check_time: Time).returns(T::Boolean) }
  def would_violate_interval?(takes:, check_time:)
    return false unless interval_limit?

    last_take = takes.select { |take| take.taken_at <= check_time }.max_by(&:taken_at)
    return false unless last_take

    hours_since_last = (check_time - last_take.taken_at) / 3600
    hours_since_last < T.cast(min_hours_between_doses, T.any(Integer, Float, Rational, BigDecimal))
  end

  sig do
    params(
      takes: T::Array[MedicationTake],
      check_time: Time,
      cycle: DoseCycle
    ).returns(T::Boolean)
  end
  def satisfied_by?(takes:, check_time:, cycle:)
    return true unless restrictions?

    !would_exceed_daily_limit?(takes: takes, cycle: cycle, check_time: check_time) &&
      !would_violate_interval?(takes: takes, check_time: check_time)
  end

  sig do
    params(takes: T::Array[MedicationTake], cycle: DoseCycle, now: Time).returns(T.nilable(Time))
  end
  def next_available_time(takes:, cycle:, now:)
    return nil unless restrictions?
    return now if satisfied_by?(takes: takes, check_time: now, cycle: cycle)

    [
      next_time_from_interval_limit(takes: takes),
      next_time_from_daily_limit(takes: takes, cycle: cycle, now: now)
    ].compact.max
  end

  private

  sig do
    params(value: T.nilable(T.any(Integer, String, BigDecimal))).returns(T.nilable(T.any(Integer, String, BigDecimal)))
  end
  def normalize(value)
    value unless value == ''
  end

  sig { params(takes: T::Array[MedicationTake]).returns(T.nilable(Time)) }
  def next_time_from_interval_limit(takes:)
    return nil unless interval_limit?

    last_take = takes.max_by(&:taken_at)
    return nil unless last_take

    last_take.taken_at + (T.cast(min_hours_between_doses, T.any(Integer, Float, Rational, BigDecimal)) * 3600)
  end

  sig do
    params(takes: T::Array[MedicationTake], cycle: DoseCycle, now: Time).returns(T.nilable(Time))
  end
  def next_time_from_daily_limit(takes:, cycle:, now:)
    return nil unless daily_limit?
    return nil unless would_exceed_daily_limit?(takes: takes, cycle: cycle, check_time: now)

    cycle.next_reset_time(now)
  end
end
