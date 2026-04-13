# frozen_string_literal: true

class MedicationDosageOption < ApplicationRecord
  self.table_name = 'dosages'

  belongs_to :medication

  before_validation :align_unit_with_medication
  after_create :sync_medication_dosage

  enum :default_dose_cycle, { daily: 0, weekly: 1, monthly: 2 }, prefix: :default

  scope :adult_default,  -> { where(default_for_adults: true) }
  scope :child_default,  -> { where(default_for_children: true) }

  validates :amount, presence: true, numericality: { greater_than: 0 }
  validates :unit, presence: true
  validates :frequency, presence: true
  validates :default_max_daily_doses, presence: true, numericality: { greater_than: 0 }
  validates :default_min_hours_between_doses, presence: true, numericality: { greater_than_or_equal_to: 0 }
  validates :default_dose_cycle, presence: true

  def to_value
    MedicationDosage.new(
      amount: amount,
      unit: medication&.dosage_unit.presence || unit,
      frequency: frequency,
      description: description,
      default_for_adults: default_for_adults,
      default_for_children: default_for_children,
      default_max_daily_doses: default_max_daily_doses,
      default_min_hours_between_doses: default_min_hours_between_doses,
      default_dose_cycle: default_dose_cycle
    )
  end

  private

  def align_unit_with_medication
    self.unit = medication&.dosage_unit.presence || unit
  end

  def sync_medication_dosage
    stmt = 'UPDATE medications SET dosage_amount = NULL WHERE id = $1'
    binds = [
      ActiveRecord::Relation::QueryAttribute.new('id', medication_id, ActiveRecord::Type::BigInteger.new)
    ]
    ActiveRecord::Base.connection.exec_update(stmt, 'Sync Medication Dosage', binds)
  end
end
