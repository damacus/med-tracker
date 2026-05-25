# frozen_string_literal: true

class ScheduleFrequencyPhrase
  CYCLE_KEYS = {
    'daily' => :day,
    'weekly' => :week,
    'monthly' => :month
  }.freeze

  attr_reader :max_daily_doses, :min_hours_between_doses, :dose_cycle

  def initialize(max_daily_doses:, min_hours_between_doses:, dose_cycle:)
    @max_daily_doses = normalize_number(max_daily_doses)
    @min_hours_between_doses = normalize_number(min_hours_between_doses)
    @dose_cycle = DoseCycle.new(dose_cycle).to_s
  end

  def to_s
    parts = []
    parts << dose_limit_phrase if max_daily_doses.present?
    parts << spacing_phrase(parts.any?) if min_hours_between_doses.present?
    parts.join(', ')
  end

  private

  def dose_limit_phrase
    if max_daily_doses == 1
      I18n.t('schedules.frequency_phrase.once_per_cycle', cycle: cycle_name)
    else
      I18n.t('schedules.frequency_phrase.up_to_per_cycle', count: format_number(max_daily_doses), cycle: cycle_name)
    end
  end

  def spacing_phrase(lowercase)
    key = lowercase ? :with_minimum_spacing : :minimum_spacing
    I18n.t("schedules.frequency_phrase.#{key}", duration: duration_phrase)
  end

  def duration_phrase
    if whole_minutes?
      I18n.t('schedules.frequency_phrase.durations.minutes', count: format_number(min_hours_between_doses * 60))
    else
      I18n.t('schedules.frequency_phrase.durations.hours', count: format_number(min_hours_between_doses))
    end
  end

  def whole_minutes?
    min_hours_between_doses < 1 && (min_hours_between_doses * 60) == (min_hours_between_doses * 60).to_i
  end

  def cycle_name
    I18n.t("schedules.frequency_phrase.cycles.#{CYCLE_KEYS.fetch(dose_cycle)}")
  end

  def normalize_number(value)
    return if value.blank?

    BigDecimal(value.to_s)
  rescue ArgumentError
    nil
  end

  def format_number(value)
    value.to_i == value ? value.to_i : value.to_f
  end
end
