# frozen_string_literal: true

class StorageMigrationRun < ApplicationRecord
  SERVICES = %w[persistent s3].freeze

  before_validation { self.run_id ||= SecureRandom.uuid }

  enum :phase, {
    backfill: 'backfill',
    reconciled: 'reconciled',
    rollback_window: 'rollback_window',
    finalized: 'finalized'
  }, validate: true

  validates :run_id, presence: true, uniqueness: true
  validates :source_service_name, :destination_service_name, inclusion: { in: SERVICES }
  validates :processed_count, :verified_count, :failed_count,
            numericality: { only_integer: true, greater_than_or_equal_to: 0 }
  validate :services_differ

  def finalization_attributes(timestamp)
    {
      phase: :finalized,
      acceptance_verified_at: timestamp,
      recovery_verified_at: timestamp,
      final_reconciled_at: timestamp,
      finalized_at: timestamp
    }
  end

  private

  def services_differ
    return if source_service_name.blank? || destination_service_name.blank?
    return unless source_service_name == destination_service_name

    errors.add(:destination_service_name, 'must differ from source service')
  end
end
