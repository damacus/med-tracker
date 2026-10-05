# frozen_string_literal: true

# typed: true

require 'sorbet-runtime'

class ScheduleFrequencyPhrase
  extend T::Sig

  CYCLE_KEYS = T.let({
    'daily' => :day,
    'weekly' => :week,
    'monthly' => :month
  }.freeze, T::Hash[String, Symbol])

  sig { returns(T.nilable(BigDecimal)) }
  attr_reader :max_daily_doses

  sig { returns(T.nilable(BigDecimal)) }
  attr_reader :min_hours_between_doses

  sig { returns(String) }
  attr_reader :dose_cycle

  sig do
    params(
      max_daily_doses: T.untyped,
      min_hours_between_doses: T.untyped,
      dose_cycle: T.untyped
    ).void.checked(:never)
  end
  def initialize(max_daily_doses:, min_hours_between_doses:, dose_cycle:)
    @max_daily_doses = normalize_number(max_daily_doses)
    @min_hours_between_doses = normalize_number(min_hours_between_doses)
    @dose_cycle = DoseCycle.new(dose_cycle).to_s
  end

  sig { returns(String) }
  def to_s
    parts = T.let([], T::Array[String])
    parts << dose_limit_phrase if max_daily_doses
    parts << spacing_phrase(parts.any?) if min_hours_between_doses
    parts.join(', ')
  end

  private

  sig { returns(String) }
  def dose_limit_phrase
    count = T.must(max_daily_doses)
    if count == 1
      I18n.t('schedules.frequency_phrase.once_per_cycle', cycle: cycle_name)
    else
      I18n.t('schedules.frequency_phrase.up_to_per_cycle', count: format_number(count), cycle: cycle_name)
    end
  end

  sig { params(lowercase: T::Boolean).returns(String) }
  def spacing_phrase(lowercase)
    key = lowercase ? :with_minimum_spacing : :minimum_spacing
    I18n.t("schedules.frequency_phrase.#{key}", duration: duration_phrase)
  end

  sig { returns(String) }
  def duration_phrase
    hours = T.must(min_hours_between_doses)
    if whole_minutes?
      I18n.t('schedules.frequency_phrase.durations.minutes', count: format_number(hours * 60))
    else
      I18n.t('schedules.frequency_phrase.durations.hours', count: format_number(hours))
    end
  end

  sig { returns(T::Boolean) }
  def whole_minutes?
    hours = T.must(min_hours_between_doses)
    hours < 1 && (hours * 60) == (hours * 60).to_i
  end

  sig { returns(String) }
  def cycle_name
    I18n.t("schedules.frequency_phrase.cycles.#{CYCLE_KEYS.fetch(dose_cycle)}")
  end

  sig { params(value: T.untyped).returns(T.nilable(BigDecimal)).checked(:never) }
  def normalize_number(value)
    return if value.blank?

    BigDecimal(value.to_s)
  rescue ArgumentError
    nil
  end

  sig { params(value: BigDecimal).returns(T.any(Integer, Float)) }
  def format_number(value)
    comparable_value = T.unsafe(value)
    comparable_value.to_i == comparable_value ? comparable_value.to_i : comparable_value.to_f
  end
end
