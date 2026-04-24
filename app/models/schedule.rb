# frozen_string_literal: true

# Schedule model
class Schedule < ApplicationRecord
  include TimingRestrictions

  WEEKDAY_INDEXES = Date::DAYNAMES.each_with_index.with_object({}) do |(name, index), indexes|
    indexes[name.downcase] = index
    indexes[name.downcase.first(3)] = index
  end.freeze

  attr_accessor :dosage

  belongs_to :person
  belongs_to :medication
  belongs_to :source_dosage_option, class_name: 'MedicationDosageOption', optional: true

  enum :dose_cycle, { daily: 0, weekly: 1, monthly: 2 }
  enum :schedule_type, {
    daily: 0,
    multiple_daily: 1,
    weekly: 2,
    specific_dates: 3,
    prn: 4,
    tapering: 5,
    every_other_day: 6
  }, prefix: :schedule_type

  has_many :medication_takes, dependent: :destroy

  scope :active, lambda {
    where('start_date <= ? AND end_date >= ?', Time.zone.today, Time.zone.today)
  }

  validates :start_date, presence: true
  validates :end_date, presence: true
  validates :dose_amount, presence: true, numericality: { greater_than: 0 }
  validates :dose_unit, presence: true, inclusion: { in: Medication::DOSAGE_UNITS }
  validate :source_dosage_option_matches_medication
  validate :source_dosage_option_matches_snapshot
  validate :end_date_after_start_date
  before_validation :assign_source_dosage_option
  before_validation :assign_dose_snapshot_from_dosage

  delegate :out_of_stock?, to: :medication
  delegate :name, to: :medication, prefix: true
  delegate :name, to: :person, prefix: true
  delegate :amount, :unit, to: :dose_snapshot, prefix: true, allow_nil: true

  def default_dose_amount = dose_amount

  def applies_on?(date)
    date = normalize_date(date)
    return false if date.blank?
    return false unless within_schedule_range?(date)

    case schedule_type
    when 'weekly' then configured_weekday?(date)
    when 'specific_dates' then configured_date?(date)
    when 'every_other_day' then every_other_day_from_start?(date)
    when 'tapering' then current_taper_step(date).present?
    else true
    end
  end

  def expected_doses_on(date)
    date = normalize_date(date)
    return 0 if date.blank? || !applies_on?(date) || schedule_type_prn?

    configured_times.presence&.size || effective_max_daily_doses(date).presence || 1
  end

  def effective_dose_amount(date = Time.zone.today)
    decimal_config_value(effective_config_for(date), 'amount', 'dose_amount') || dose_amount
  end

  def effective_dose_unit(date = Time.zone.today)
    config_value(effective_config_for(date), 'unit', 'dose_unit') || dose_unit
  end

  def effective_max_daily_doses(date = Time.zone.today)
    integer_config_value(effective_config_for(date), 'max_daily_doses', 'max_doses', 'max') || max_daily_doses
  end

  def effective_min_hours_between_doses(date = Time.zone.today)
    numeric_config_value(effective_config_for(date), 'min_hours_between_doses', 'min_hours', 'minimum_hours') ||
      min_hours_between_doses
  end

  def active?
    today = Time.zone.today
    return false if start_date.nil? || end_date.nil?

    today.between?(start_date, end_date)
  end

  def cycle_period
    DoseCycle.new(dose_cycle).period
  end

  def dose_display
    dose_snapshot&.to_s
  end

  def dose_snapshot
    return if dose_amount.blank? || dose_unit.blank?

    MedicationDosage.new(
      amount: dose_amount,
      unit: dose_unit,
      frequency: frequency,
      description: nil,
      default_for_adults: false,
      default_for_children: false,
      default_max_daily_doses: max_daily_doses,
      default_min_hours_between_doses: min_hours_between_doses,
      default_dose_cycle: dose_cycle
    )
  end

  private

  def assign_source_dosage_option
    self.source_dosage_option ||= dosage if dosage.is_a?(MedicationDosageOption)
    self.source_dosage_option ||= uniquely_matching_dosage_option
  end

  def assign_dose_snapshot_from_dosage
    return if dose_amount.present? && dose_unit.present?

    resolved_dosage = source_dosage_option || dosage
    return if resolved_dosage.blank?

    self.dose_amount ||= resolved_dosage.amount
    self.dose_unit ||= resolved_dosage.unit
  end

  def end_date_after_start_date
    return if end_date.blank? || start_date.blank?

    return unless end_date < start_date

    errors.add(:end_date, 'must be after the start date')
  end

  def source_dosage_option_matches_medication
    return if source_dosage_option.blank? || medication.blank?
    return if source_dosage_option.medication_id == medication_id

    errors.add(:source_dosage_option, 'must belong to the selected medication')
  end

  def source_dosage_option_matches_snapshot
    return if source_dosage_option.blank? || dose_amount.blank? || dose_unit.blank?
    return if source_dosage_option.amount.to_s == dose_amount.to_s && source_dosage_option.unit == dose_unit

    errors.add(:source_dosage_option, 'must match the selected dose')
  end

  def uniquely_matching_dosage_option
    return if medication.blank? || dose_amount.blank? || dose_unit.blank?

    matches = medication.dosage_records.where(amount: dose_amount, unit: dose_unit)
    return matches.first if matches.one?

    nil
  end

  def within_schedule_range?(date)
    return false if start_date.blank? || end_date.blank?

    date.between?(start_date, end_date)
  end

  def configured_weekday?(date)
    Array(config_value(schedule_config_hash, 'weekdays')).any? { |weekday| weekday_matches?(weekday, date) }
  end

  def configured_date?(date)
    dates = Array(config_value(schedule_config_hash, 'dates')).filter_map do |configured_date|
      normalize_date(configured_date)
    end
    dates.include?(date)
  end

  def every_other_day_from_start?(date) = ((date - start_date).to_i % 2).zero?

  def configured_times = Array(config_value(schedule_config_hash, 'times')).compact_blank

  def effective_config_for(date)
    date = normalize_date(date)
    return schedule_config_hash unless schedule_type_tapering? && date.present?

    current_taper_step(date) || schedule_config_hash
  end

  def current_taper_step(date)
    Array(config_value(schedule_config_hash, 'taper_steps')).find do |step|
      step_applies_on?(step, date)
    end
  end

  def step_applies_on?(step, date)
    step_start = normalize_date(config_value(step, 'start_date'))
    step_end = normalize_date(config_value(step, 'end_date'))
    return false if step_start.blank? || step_end.blank?

    date.between?(step_start, step_end)
  end

  def weekday_matches?(weekday, date)
    weekday_index = normalize_weekday(weekday)
    return false if weekday_index.blank?

    weekday_index == date.wday || weekday_index == date.cwday
  end

  def normalize_weekday(weekday)
    return weekday if weekday.is_a?(Integer)

    weekday = weekday.to_s.strip.downcase
    return if weekday.blank?
    return weekday.to_i if weekday.match?(/\A\d+\z/)

    WEEKDAY_INDEXES[weekday]
  end

  def normalize_date(value)
    return value if value.is_a?(Date)
    return value.to_date if value.respond_to?(:to_date) && !value.is_a?(String)

    Date.iso8601(value.to_s)
  rescue Date::Error, TypeError
    nil
  end

  def schedule_config_hash = schedule_config || {}

  def config_value(hash, *keys)
    return if hash.blank?

    keys.each do |key|
      return hash[key.to_s] if hash.key?(key.to_s)
      return hash[key.to_sym] if hash.key?(key.to_sym)
    end

    nil
  end

  def decimal_config_value(hash, *keys)
    value = config_value(hash, *keys)
    return if value.blank?

    BigDecimal(value.to_s)
  rescue ArgumentError
    nil
  end

  def integer_config_value(hash, *keys) = numeric_config_value(hash, *keys)&.to_i

  def numeric_config_value(hash, *keys)
    value = config_value(hash, *keys)
    return if value.blank?

    BigDecimal(value.to_s)
  rescue ArgumentError
    nil
  end
end
