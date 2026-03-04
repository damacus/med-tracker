# frozen_string_literal: true

class Schedule < ApplicationRecord
  belongs_to :person
  belongs_to :medication
  belongs_to :dosage, optional: true
  has_many :medication_administrations, dependent: :destroy

  validates :start_date, presence: true
  validates :frequency, presence: true
  validates :custom_dose_amount, presence: true, if: -> { dosage_id.blank? }
  validates :custom_dose_unit, presence: true, if: -> { dosage_id.blank? }
  validate :at_least_one_dose_source

  scope :active, -> { where('start_date <= ? AND (end_date IS NULL OR end_date >= ?)', Time.zone.today, Time.zone.today) }

  delegate :name, to: :medication, prefix: true
  delegate :name, to: :person, prefix: true
  delegate :amount, :unit, to: :dosage, prefix: true, allow_nil: true

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
    # Mock implementation for now
    7.days
  end

  private

  def at_least_one_dose_source
    return if dosage_id.present? || (custom_dose_amount.present? && custom_dose_unit.present?)

    errors.add(:base, 'Must specify either a preset dosage or a custom dose amount and unit')
  end
end
