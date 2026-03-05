# frozen_string_literal: true

class Schedule < ApplicationRecord
  include TimingRestrictions

  belongs_to :person
  belongs_to :medication
  belongs_to :dosage, optional: true
  has_many :medication_takes, dependent: :destroy

  validates :start_date, presence: true
  validates :frequency, presence: true
  validates :end_date, presence: true
  validates :custom_dose_amount, presence: true, numericality: { greater_than: 0, message: 'must be greater than zero' }, if: -> { dosage_id.blank? }
  validates :custom_dose_unit, presence: true, if: -> { dosage_id.blank? }
  validate :at_least_one_dose_source
  validate :end_date_after_start_date

  before_validation :set_default_frequency, on: :create

  scope :active, lambda {
    where('start_date <= ? AND (end_date IS NULL OR end_date >= ?)', Time.zone.today, Time.zone.today)
  }

  delegate :name, to: :medication, prefix: true
  delegate :name, to: :person, prefix: true
  delegate :amount, :unit, to: :dosage, prefix: true, allow_nil: true
  delegate :out_of_stock?, :low_stock?, :current_supply, to: :medication, allow_nil: true

  enum :dose_cycle, { daily: 0, weekly: 1, monthly: 2 }, prefix: :dose
  enum :schedule_type, { scheduled: 'scheduled', as_needed: 'as_needed' }, default: 'scheduled'

  def active?
    today = Time.zone.today
    return false if start_date.nil?
    return today >= start_date if end_date.nil?

    today.between?(start_date, end_date)
  end

  def effective_dose_amount
    dosage ? dosage.amount : custom_dose_amount
  end

  def effective_dose_unit
    dosage ? dosage.unit : custom_dose_unit
  end

  def dosage_text
    "#{effective_dose_amount} #{effective_dose_unit}"
  end

  def cycle_period
    case dose_cycle
    when 'weekly' then 1.week
    when 'monthly' then 1.month
    else 1.day
    end
  end

  private

  def end_date_after_start_date
    return if end_date.blank? || start_date.blank?

    errors.add(:end_date, 'must be after the start date') if end_date < start_date
  end

  def at_least_one_dose_source
    return if dosage_id.present? || (custom_dose_amount.present? && custom_dose_unit.present?)

    errors.add(:base, 'Either a preset dosage or a custom amount and unit must be provided')
  end

  def set_default_frequency
    self.frequency ||= 'daily'
  end
end
