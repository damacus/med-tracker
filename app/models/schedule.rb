# frozen_string_literal: true

# Schedule model
class Schedule < ApplicationRecord
  include TimingRestrictions

  attr_accessor :dosage

  belongs_to :person
  belongs_to :medication

  enum :dose_cycle, { daily: 0, weekly: 1, monthly: 2 }

  has_many :medication_takes, dependent: :destroy

  scope :active, lambda {
    where('start_date <= ? AND end_date >= ?', Time.zone.today, Time.zone.today)
  }

  validates :start_date, presence: true
  validates :end_date, presence: true
  validates :dose_amount, presence: true, numericality: { greater_than: 0 }
  validates :dose_unit, presence: true, inclusion: { in: Medication::DOSAGE_UNITS }
  validate :end_date_after_start_date
  before_validation :assign_dose_snapshot_from_dosage

  delegate :out_of_stock?, to: :medication
  delegate :name, to: :medication, prefix: true
  delegate :name, to: :person, prefix: true
  delegate :amount, :unit, to: :dose_snapshot, prefix: true, allow_nil: true

  def default_dose_amount
    dose_amount
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

  def assign_dose_snapshot_from_dosage
    return if dose_amount.present? && dose_unit.present?
    return if dosage.blank?

    self.dose_amount ||= dosage.amount
    self.dose_unit ||= dosage.unit
  end

  def end_date_after_start_date
    return if end_date.blank? || start_date.blank?

    return unless end_date < start_date

    errors.add(:end_date, 'must be after the start date')
  end
end
