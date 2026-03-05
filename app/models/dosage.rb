# frozen_string_literal: true

class Dosage < ApplicationRecord
  belongs_to :medication
  has_many :schedules, dependent: :destroy

  after_create :sync_medication_dosage

  enum :default_dose_cycle, { daily: 0, weekly: 1, monthly: 2 }, prefix: :default

  scope :adult_default,  -> { where(default_for_adults: true) }
  scope :child_default,  -> { where(default_for_children: true) }

  validates :amount, presence: true, numericality: { greater_than: 0 }
  validates :unit, presence: true
  validates :frequency, presence: true

  private

  def sync_medication_dosage
    # When a multi-dose record is created, the medication's "standard dosage"
    # fields must be cleared to ensure only one mode is active at a time.
    # This prevents UI confusion and data inconsistency.
    # Uses SQL to comply with RuboCop and project standards.
    stmt = 'UPDATE medications SET dosage_amount = NULL, dosage_unit = NULL WHERE id = $1'
    binds = [
      ActiveRecord::Relation::QueryAttribute.new('id', medication_id, ActiveRecord::Type::BigInteger.new)
    ]
    ActiveRecord::Base.connection.exec_update(stmt, 'Sync Medication Dosage', binds)
  end
end
